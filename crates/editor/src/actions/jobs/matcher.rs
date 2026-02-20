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
use tokio::sync::{
    mpsc::{channel, Sender},
    Mutex,
};
use tokio_util::sync::CancellationToken;

use crate::{
    actions::jobs::CHANNEL_SIZE,
    common::choice::{Choice, ScoredChoice},
    editor::{job_broker::KeepInTouch, Editor},
};

use sanedit_server::{ClientId, Job, JobContext, JobResult};

use std::cmp::min;

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
    fn run(&self, ctx: JobContext) -> JobResult {
        let strat = self.strat;
        let opts = self.opts.clone();
        let starting_pattern = self.pattern.clone();

        let fut = async move {
            // Term channel
            let (psend, mut precv) = channel::<(String, u64)>(CHANNEL_SIZE);
            let list = Appendlist::<Arc<Choice>>::new();

            ctx.send(MatcherMessage::Init(psend));

            let kill = ctx.kill.clone();
            let matcher = Arc::new(Matcher::new(list.clone(), strat, ctx));
            let write_done = matcher.write_done();
            let do_matching = async move {
                let token = CancellationToken::new();
                let ctoken = token.clone();
                let mmatch = matcher.clone();
                let handle = tokio::spawn(async move {
                    let _ = tokio::task::spawn_blocking(move || {
                        mmatch.do_match(&starting_pattern, 0, ctoken)
                    })
                    .await;
                });
                let task = Arc::new(Mutex::new(Some((token, handle))));

                while let Some((pattern, input_id)) = precv.recv().await {
                    let mut guard = task.lock().await;
                    if let Some((token, handle)) = guard.take() {
                        token.cancel();
                        let _ = handle.await;
                    }

                    let token = CancellationToken::new();
                    let ctoken = token.clone();
                    let mmatch = matcher.clone();
                    let handle = tokio::spawn(async move {
                        let _ = tokio::task::spawn_blocking(move || {
                            mmatch.do_match(&pattern, input_id, ctoken)
                        })
                        .await;
                    });

                    *guard = Some((token, handle));
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
#[derive(Clone)]
pub struct Matcher {
    reader: Appendlist<Arc<Choice>>,
    write_done: Arc<AtomicUsize>,
    strategy: MatchStrategy,
    job_context: JobContext,
}

impl Matcher {
    const BATCH_SIZE: usize = 512;

    // Create a new matcher.
    pub fn new(
        reader: Appendlist<Arc<Choice>>,
        strategy: MatchStrategy,
        job_context: JobContext,
    ) -> Matcher {
        let write_done = Arc::new(AtomicUsize::new(usize::MAX));

        Matcher {
            reader,
            write_done,
            strategy,
            job_context,
        }
    }

    pub fn write_done(&self) -> Arc<AtomicUsize> {
        self.write_done.clone()
    }

    /// Match the candidates with the pattern. Returns a receiver where the results
    /// can be read from in chunks.
    /// Dropping the receiver stops the matching process.
    pub fn do_match(&self, pattern: &str, id: u64, cancel: CancellationToken) {
        let reader = self.reader.clone();
        let write_done = &self.write_done;
        let sender = &self.job_context;
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

        const INITIAL_BACKOFF: u64 = 1;
        let mut backoff = INITIAL_BACKOFF;
        let mut taken = 0;
        let send_rate = Duration::from_millis(1000 / 30);
        let mut last_sent = Instant::now();
        let mut first_sent = true;
        let mut locally_sorted = vec![];

        log::info!("Matching options");

        while !self.job_context.kill.should_stop() && !cancel.is_cancelled() {
            let total = write_done.load(Ordering::Acquire);
            let available = reader.len();
            let fully_read = available == total;

            // If we are done reading all available options
            if fully_read && available == taken {
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
                    .iter()
                    .filter_map(|choice| {
                        let ranges = matches_with(choice.filter_text(), &matcher)?;
                        let score = score(choice, &ranges);
                        let scored = ScoredChoice::new(choice.clone(), score, ranges);
                        Some(scored)
                    })
                    .collect();

                results.sort();
                locally_sorted.push(unsafe { SortedVec::from_sorted_unchecked(results) });

                if last_sent.elapsed() > send_rate {
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

        log::info!("Matching options done");
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

#[cfg(test)]
mod test {
    use crossbeam::channel::Receiver;
    use sanedit_server::{FromJobs, ToEditor};

    use super::*;

    fn assert_contains(recv: &mut Receiver<ToEditor>, input: u64, choices: &[&str]) {
        let mut all = std::collections::HashSet::new();
        for choice in choices {
            all.insert(choice.to_string());
        }

        while let Ok(res) = recv.recv() {
            let ToEditor::Jobs(FromJobs::Message(_, any)) = res else {
                panic!("Invalid message")
            };

            let msg = any.downcast::<MatcherMessage>();
            assert!(msg.is_ok());
            let Ok(msg) = msg else { panic!("Invalid cast") };
            match msg.as_ref() {
                MatcherMessage::Init(_) => {}
                MatcherMessage::Done {
                    input_id,
                    results,
                    ..
                } => {
                    assert_eq!(input, *input_id);
                    for rv in results {
                        for res in rv.iter() {
                            assert!(all.remove(res.choice().text()));
                        }
                    }
                    break;
                }
                MatcherMessage::Progress {
                    input_id,
                    results,
                    ..
                } => {
                    assert_eq!(input, *input_id);
                    for rv in results {
                        for res in rv.iter() {
                            assert!(all.remove(res.choice().text()));
                        }
                    }
                }
            }
        }

        assert!(all.is_empty());
    }

    fn make_list(choices: &[&str]) -> Appendlist<Arc<Choice>> {
        let list = Appendlist::new();
        for choice in choices {
            list.append(Choice::from_text(choice.to_string()));
        }
        list
    }

    #[test]
    fn matches_options() {
        let (ctx, mut recv) = JobContext::new_test();
        let list = make_list(&[
            "/run/socket.sock",
            "/var/log/syslog",
            "/tmp/abc.tmp",
            "/home/me/.ssh/config",
            "/home/me/.config/config.toml",
        ]);

        let matcher = Matcher::new(list.clone(), MatchStrategy::Default, ctx);
        matcher.write_done.store(list.len(), Ordering::Release);
        matcher.do_match("home", 0, CancellationToken::default());
        assert_contains(
            &mut recv,
            0,
            &["/home/me/.ssh/config", "/home/me/.config/config.toml"],
        );

        matcher.do_match("sock", 1, CancellationToken::default());
        assert_contains(&mut recv, 1, &["/run/socket.sock"]);
    }
}
