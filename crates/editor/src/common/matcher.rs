mod matches;
mod receiver;
mod strategy;

use std::{
    borrow::Cow,
    cmp::min,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

use sanedit_core::Range;
use sanedit_utils::appendlist::Reader;
use tokio::sync::mpsc::channel;

pub(crate) use matches::*;
pub use receiver::*;
pub use strategy::*;

/// Matches options to a pattern
pub struct Matcher {
    reader: Reader<Arc<Choice>>,
    read_done: Arc<AtomicUsize>,
    prev_search: Arc<AtomicBool>,
    strategy: MatchStrategy,
}

impl Matcher {
    const BATCH_SIZE: usize = 1024;
    const CHANNEL_SIZE: usize = 1024;

    // Create a new matcher.
    pub fn new(reader: Reader<Arc<Choice>>, strategy: MatchStrategy) -> Matcher {
        let read_done = Arc::new(AtomicUsize::new(usize::MAX));
        let prev_search = Arc::new(AtomicBool::new(false));

        Matcher {
            reader,
            read_done,
            strategy,
            prev_search,
        }
    }

    pub fn read_done(&self) -> Arc<AtomicUsize> {
        self.read_done.clone()
    }

    /// Match the candidates with the pattern. Returns a receiver where the results
    /// can be read from in chunks.
    /// Dropping the receiver stops the matching process.
    pub fn do_match(&mut self, pattern: &str) -> MatchReceiver {
        // Cancel possible previous search
        self.prev_search.store(true, Ordering::Release);
        self.prev_search = Arc::new(AtomicBool::new(false));

        // Batch candidates
        // Send each block to an executor
        // Get the results and send to receiver
        let (out, rx) = channel::<ScoredChoice>(Self::CHANNEL_SIZE);
        let reader = self.reader.clone();
        let read_done = self.read_done.clone();
        let case_sensitive = pattern.chars().any(|ch| ch.is_uppercase());
        let strat = self.strategy;
        let match_fn = strat.get();

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
        let mut taken = 0;
        let local_stop = self.prev_search.clone();

        rayon::spawn(move || loop {
            if local_stop.load(Ordering::Acquire) {
                break;
            }

            let total = read_done.load(Ordering::Acquire);
            let available = reader.len();
            let fully_read = available == total;

            // If we are done reading all available options
            if fully_read && available == taken {
                break;
            }

            if available >= taken + Self::BATCH_SIZE || fully_read {
                let size = min(available - taken, Self::BATCH_SIZE);
                let batch = taken..taken + size;
                taken += size;

                let out = out.clone();
                let stop = local_stop.clone();
                let reader = reader.clone();
                let patterns = patterns.clone();

                rayon::spawn(move || {
                    let opts = reader.slice(batch);
                    for choice in opts.iter() {
                        if stop.load(Ordering::Acquire) {
                            return;
                        }

                        if let Some(ranges) = matches_with(
                            choice.filter_text().as_ref(),
                            &patterns,
                            case_sensitive,
                            match_fn,
                        ) {
                            let score = choice
                                .number()
                                .unwrap_or_else(|| score(&choice.filter_text(), &ranges));
                            let scored = ScoredChoice::new(choice.clone(), score, ranges);

                            if out.blocking_send(scored).is_err() {
                                stop.store(true, Ordering::Release);
                                return;
                            }
                        }
                    }
                });
            }
        });

        MatchReceiver { receiver: rx }
    }
}

// Score a match
fn score(opt: &str, ranges: &[Range<usize>]) -> u32 {
    // Closest match first
    // Shortest item first
    ranges.first().map(|f| f.start).unwrap_or(opt.len()) as u32
    // let match_at = ranges.first().map(|f| f.start as u16).unwrap_or(0);
    // let len = opt.len() as u16;
    // ((match_at as u32) << u16::BITS) | len as u32
}

fn with_case_sensitivity(opt: &str, case_sensitive: bool) -> Cow<str> {
    if case_sensitive {
        return opt.into();
    }

    let has_upper = opt.chars().any(|ch| ch.is_ascii_uppercase());
    if !has_upper {
        return opt.into();
    }

    Cow::from(opt.to_ascii_lowercase())
}

fn matches_with(
    opt: &str,
    patterns: &Arc<Vec<String>>,
    case_sensitive: bool,
    f: fn(&str, &str) -> Option<Range<usize>>,
) -> Option<Vec<Range<usize>>> {
    let string: Cow<str> = with_case_sensitivity(opt, case_sensitive);
    let mut matches = vec![];
    for pattern in patterns.iter() {
        // Calculate match and apply offset
        let range = (f)(&string, pattern)?;
        matches.push(range);
    }

    Some(matches)
}
