mod option_provider;
mod strategy;

use std::{
    cmp::min,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use crossbeam::channel::{Receiver, TryRecvError};
pub(crate) use option_provider::{Empty, OptionProvider};
use sanedit_utils::{appendlist::Appendlist, sorted_vec::SortedVec};

use crate::{
    actions::jobs::CHANNEL_SIZE,
    common::choice::{Choice, ScoredChoice},
    editor::{job_broker::KeepInTouch, Editor},
};

use sanedit_server::{ClientId, Job, JobContext, JobResult};

use sanedit_core::Range;

pub use strategy::*;

#[derive(Debug)]
pub(crate) enum MatcherMessage {
    Init(tokio::sync::mpsc::Sender<(String, u64)>),
    Done {
        input_id: u64,
        clear_old: bool,
        results: Vec<SortedVec<ScoredChoice>>,
    },
    Progress {
        input_id: u64,
        clear_old: bool,
        results: Vec<SortedVec<ScoredChoice>>,
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
            let (pattern_send, mut pattern_recv) =
                tokio::sync::mpsc::channel::<(String, u64)>(CHANNEL_SIZE);
            let (option_send, option_recv) = crossbeam::channel::unbounded::<Arc<Choice>>();
            let option_list = Appendlist::<Arc<Choice>>::new();

            ctx.send(MatcherMessage::Init(pattern_send));

            let matcher = Arc::new(Matcher::new(option_recv, strat));
            let do_matching = async move {
                log::info!("do_matching started");
                let mut cancel = Arc::new(AtomicBool::new(false));

                let result_sender = ctx.clone();
                let list = option_list.clone();
                let nmatcher = matcher.clone();
                let ccancel = cancel.clone();

                rayon::spawn(move || {
                    nmatcher.do_match(0, &starting_pattern, list, result_sender, ccancel);
                });

                while let Some((pattern, input_id)) = pattern_recv.recv().await {
                    log::info!("do_matching new pattern: {pattern}");
                    cancel.store(true, Ordering::Release);
                    cancel = Arc::new(AtomicBool::new(false));

                    let result_sender = ctx.clone();
                    let list = option_list.clone();
                    let nmatcher = matcher.clone();
                    let ccancel = cancel.clone();

                    rayon::spawn(move || {
                        nmatcher.do_match(input_id, &pattern, list, result_sender, ccancel);
                    });
                }

                log::info!("do_matching finished");
            };

            tokio::join!(opts.provide(option_send), do_matching);

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
    receiver: Receiver<Arc<Choice>>,
    strategy: MatchStrategy,
}

impl Matcher {
    // Create a new matcher.
    pub fn new(receiver: Receiver<Arc<Choice>>, strategy: MatchStrategy) -> Matcher {
        Matcher { receiver, strategy }
    }

    pub fn do_match(
        &self,
        input_id: u64,
        pattern: &str,
        option_list: Appendlist<Arc<Choice>>,
        result_sender: JobContext,
        cancel: Arc<AtomicBool>,
    ) {
        let case_sensitive = pattern.chars().any(|ch| ch.is_uppercase());
        let strat = self.strategy;
        let mut is_first_result = true;

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

        const BATCH_SIZE: usize = 512;
        const INITIAL_BACKOFF: u64 = 50;
        let send_rate = Duration::from_millis(1000 / 30);
        let mut backoff = INITIAL_BACKOFF;
        let mut taken = 0;
        let mut last_sent_results = Instant::now();
        let mut all_options_received = false;
        let mut locally_sorted = vec![];

        log::info!("Matching options");

        while !result_sender.kill.should_stop() && !cancel.load(Ordering::Acquire) {
            loop {
                match self.receiver.try_recv() {
                    Ok(opt) => option_list.append(opt),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        all_options_received = true;
                        break;
                    }
                }
            }

            let available = option_list.len();

            if all_options_received && available == taken {
                result_sender.send(MatcherMessage::Done {
                    input_id,
                    clear_old: is_first_result,
                    results: std::mem::take(&mut locally_sorted),
                });
                break;
            }

            if available >= taken + BATCH_SIZE || all_options_received {
                backoff = INITIAL_BACKOFF;
                let size = min(available - taken, BATCH_SIZE);
                let batch = taken..taken + size;
                taken += size;

                let opts = option_list.slice(batch);
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

                if last_sent_results.elapsed() > send_rate {
                    result_sender.send(MatcherMessage::Progress {
                        input_id,
                        clear_old: is_first_result,
                        results: std::mem::take(&mut locally_sorted),
                    });
                    last_sent_results = Instant::now();
                    is_first_result = false;
                }
            } else {
                std::thread::sleep(Duration::from_micros(backoff));
                backoff = (backoff * 2).min(500);
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
                    input_id, results, ..
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
                    input_id, results, ..
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
        let (ctx, mut ctx_recv) = JobContext::new_test();
        let list = make_list(&[
            "/run/socket.sock",
            "/var/log/syslog",
            "/tmp/abc.tmp",
            "/home/me/.ssh/config",
            "/home/me/.config/config.toml",
        ]);

        let (send, recv) = crossbeam::channel::unbounded();

        for i in 0..list.len() {
            let _ = send.send(list.get(i).unwrap().clone());
        }
        drop(send);

        let list = Appendlist::new();
        let matcher = Matcher::new(recv, MatchStrategy::Default);
        matcher.do_match(
            0,
            "home",
            list.clone(),
            ctx.clone(),
            Arc::new(AtomicBool::new(false)),
        );
        assert_contains(
            &mut ctx_recv,
            0,
            &["/home/me/.ssh/config", "/home/me/.config/config.toml"],
        );

        matcher.do_match(1, "sock", list, ctx, Arc::new(AtomicBool::new(false)));
        assert_contains(&mut ctx_recv, 1, &["/run/socket.sock"]);
    }
}
