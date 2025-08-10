mod option_provider;
mod strategy;

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

pub(crate) use option_provider::{Empty, OptionProvider};
use sanedit_utils::{appendlist::Appendlist, sorted_vec::SortedVec};
use tokio::sync::mpsc::{channel, Sender};

use crate::{
    actions::jobs::CHANNEL_SIZE,
    common::choice::{Choice, ScoredChoice},
    editor::{job_broker::KeepInTouch, Editor},
};

use sanedit_server::{ClientId, Job, JobContext, JobResult};

use std::{cmp::min, sync::atomic::AtomicBool};

use rayon::{
    iter::{IntoParallelRefIterator as _, ParallelIterator as _},
    slice::ParallelSliceMut as _,
};
use sanedit_core::Range;

pub use strategy::*;

#[derive(Debug)]
pub(crate) enum MatcherMessage {
    Init(Sender<(String, u64)>),
    Done {
        input_id: u64,
        results: Vec<SortedVec<ScoredChoice>>,
        clear_old: bool,
    },
    Progress {
        input_id: u64,
        results: Vec<SortedVec<ScoredChoice>>,
        clear_old: bool,
    },
}

/// What to do with the matched results
pub(crate) type MatchResultHandler = fn(&mut Editor, ClientId, MatcherMessage);

pub(crate) struct MatcherJobBuilder {
    cid: ClientId,
    opts: Arc<dyn OptionProvider>,
    strat: MatchStrategy,
    result_handler: MatchResultHandler,
    pattern: String,
}

impl MatcherJobBuilder {
    pub fn new(cid: ClientId) -> MatcherJobBuilder {
        MatcherJobBuilder {
            cid,
            opts: Arc::new(Empty),
            strat: MatchStrategy::default(),
            result_handler: Empty::none_result_handler,
            pattern: String::new(),
        }
    }

    pub fn options<O: OptionProvider + 'static>(mut self, o: O) -> Self {
        self.opts = Arc::new(o);
        self
    }

    pub fn strategy(mut self, strat: MatchStrategy) -> Self {
        self.strat = strat;
        self
    }

    pub fn handler(mut self, handler: MatchResultHandler) -> Self {
        self.result_handler = handler;
        self
    }

    /// Search term to use when starting matching
    pub fn search(mut self, pattern: String) -> Self {
        self.pattern = pattern;
        self
    }

    pub fn build(self) -> MatcherJob {
        let MatcherJobBuilder {
            cid,
            opts,
            strat,
            result_handler,
            pattern: search_term,
        } = self;

        MatcherJob {
            cid,
            strat,
            result_handler,
            opts,
            pattern: search_term,
        }
    }
}

/// Matches options provided by OptionProvider
/// against a term using a matching strategy.
///
/// Matches are reported to match result handler.
#[derive(Debug, Clone)]
pub(crate) struct MatcherJob {
    cid: ClientId,

    /// Provides the options to match against
    opts: Arc<dyn OptionProvider>,

    /// Alters the behavior of matcher
    strat: MatchStrategy,

    /// Handles match results
    result_handler: MatchResultHandler,

    /// Initial search term to use
    pattern: String,
}

impl MatcherJob {
    pub fn builder(cid: ClientId) -> MatcherJobBuilder {
        MatcherJobBuilder::new(cid)
    }
}

impl Job for MatcherJob {
    fn run(&self, mut ctx: JobContext) -> JobResult {
        let strat = self.strat;
        let opts = self.opts.clone();
        let mut patt = self.pattern.clone();

        let fut = async move {
            // Term channel
            let (psend, mut precv) = channel::<(String, u64)>(CHANNEL_SIZE);
            let list = Appendlist::<Arc<Choice>>::new();

            ctx.send(MatcherMessage::Init(psend));

            let kill = ctx.kill.clone();
            let mut matcher = Matcher::new(list.clone(), strat, ctx);
            let write_done = matcher.write_done();
            let do_matching = async {
                matcher.do_match(&patt, 0);

                while let Some((t, n)) = precv.recv().await {
                    if patt == t {
                        log::info!("SAME");
                        continue;
                    }
                    patt = t;
                    matcher.do_match(&patt, n);
                }
            };

            tokio::join!(opts.provide(list, kill, write_done), do_matching);

            Ok(())
        };

        Box::pin(fut)
    }
}

impl KeepInTouch for MatcherJob {
    fn client_id(&self) -> ClientId {
        self.cid
    }

    fn on_message(&self, editor: &mut Editor, msg: Box<dyn std::any::Any>) {
        if let Ok(msg) = msg.downcast::<MatcherMessage>() {
            let id = self.client_id();
            (self.result_handler)(editor, id, *msg);
        }
    }
}

/// Matches options to a pattern
pub struct Matcher {
    reader: Appendlist<Arc<Choice>>,
    write_done: Arc<AtomicUsize>,
    prev_search: Arc<AtomicBool>,
    strategy: MatchStrategy,
    job_context: JobContext,
}

impl Matcher {
    const BATCH_SIZE: usize = 1024;

    // Create a new matcher.
    pub fn new(
        reader: Appendlist<Arc<Choice>>,
        strategy: MatchStrategy,
        job_context: JobContext,
    ) -> Matcher {
        let write_done = Arc::new(AtomicUsize::new(usize::MAX));
        let prev_search = Arc::new(AtomicBool::new(false));

        Matcher {
            reader,
            write_done,
            strategy,
            prev_search,
            job_context,
        }
    }

    pub fn write_done(&self) -> Arc<AtomicUsize> {
        self.write_done.clone()
    }

    /// Match the candidates with the pattern. Returns a receiver where the results
    /// can be read from in chunks.
    /// Dropping the receiver stops the matching process.
    pub fn do_match(&mut self, pattern: &str, id: u64) {
        // Cancel possible previous search
        self.prev_search.store(true, Ordering::Release);
        self.prev_search = Arc::new(AtomicBool::new(false));

        let reader = self.reader.clone();
        let write_done = self.write_done.clone();
        let mut sender = self.job_context.clone();
        let case_sensitive = pattern.chars().any(|ch| ch.is_uppercase());
        let strat = self.strategy;

        // Apply strategy to pattern
        // Split pattern by whitespace and use the resulting patterns as independent
        // searches
        let patterns: Arc<Vec<String>> = {
            if strat.split() {
                let patterns = pattern.split_whitespace().map(String::from).collect();
                Arc::new(patterns)
            } else {
                Arc::new(vec![pattern.into()])
            }
        };
        let matcher = strat.get_match_func(&patterns, case_sensitive);
        let local_stop = self.prev_search.clone();

        rayon::spawn(move || {
            const INITIAL_BACKOFF: u64 = 1;
            let mut backoff = INITIAL_BACKOFF;
            let mut taken = 0;
            let send_rate = Duration::from_millis(1000 / 30);
            let mut last_sent = Instant::now();
            let mut first_sent = true;
            let mut locally_sorted = vec![];

            loop {
                if local_stop.load(Ordering::Acquire) {
                    break;
                }

                let total = write_done.load(Ordering::Acquire);
                let available = reader.len();
                let fully_read = available == total;

                // If we are done reading all available options
                if fully_read && available == taken {
                    if local_stop.load(Ordering::Acquire) {
                        break;
                    }
                    sender.send(MatcherMessage::Done {
                        input_id: id,
                        clear_old: first_sent,
                        results: std::mem::take(&mut locally_sorted),
                    });
                    break;
                }

                if available >= taken + Self::BATCH_SIZE || fully_read {
                    backoff = INITIAL_BACKOFF;
                    let size = min(available - taken, Self::BATCH_SIZE);
                    let batch = taken..taken + size;
                    taken += size;

                    let opts = reader.slice(batch);
                    let mut results: Vec<ScoredChoice> = opts
                        .par_iter()
                        .filter_map(|choice| {
                            let ranges = matches_with(choice.filter_text().as_ref(), &matcher)?;
                            let score = score(&choice, &ranges);
                            let scored = ScoredChoice::new(choice.clone(), score, ranges);
                            Some(scored)
                        })
                        .collect();

                    results.par_sort();
                    locally_sorted.push(unsafe { SortedVec::from_sorted_unchecked(results) });

                    if last_sent.elapsed() > send_rate {
                        if local_stop.load(Ordering::Acquire) {
                            break;
                        }
                        sender.send(MatcherMessage::Progress {
                            input_id: id,
                            clear_old: first_sent,
                            results: std::mem::take(&mut locally_sorted),
                        });
                        last_sent = Instant::now();
                        first_sent = false;
                    }
                } else {
                    thread::sleep(Duration::from_micros(backoff));
                    backoff = (backoff * 2).min(100);
                }
            }
        });
    }
}

fn score(choice: &Arc<Choice>, ranges: &[Range<usize>]) -> usize {
    if let Some(n) = choice.number() {
        return n;
    }
    // Closest match first
    ranges.first().map(|f| f.start).unwrap_or(0)
}

fn matches_with(opt: &str, matcher: &MultiMatcher) -> Option<Vec<Range<usize>>> {
    let mut matches = vec![];
    if matcher.is_empty() {
        return Some(matches);
    }

    matcher.is_match(opt, &mut matches);
    if matches.is_empty() {
        None
    } else {
        Some(matches)
    }
}
