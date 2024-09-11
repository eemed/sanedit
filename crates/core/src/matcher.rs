mod matches;
mod receiver;
mod strategy;

use std::{
    borrow::Cow,
    cmp::min,
    ops::Range,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use sanedit_utils::appendlist::{Appendlist, Reader};
use tokio::sync::mpsc::channel;

pub use matches::*;
pub use receiver::*;
pub use strategy::*;

use crate::Choice;

/// Matches options to a pattern
pub struct Matcher {
    reader: Reader<MatchOption>,
    all_opts_read: Arc<AtomicBool>,
    previous: Arc<AtomicBool>,
    strategy: MatchStrategy,
}

impl Matcher {
    const BATCH_SIZE: usize = 1024;
    const CHANNEL_SIZE: usize = 1024;

    // Create a new matcher.
    pub fn new<T>(mut chan: T, strategy: MatchStrategy) -> Matcher
    where
        T: MatchOptionReceiver<MatchOption> + Send + 'static,
    {
        let (reader, writer) = Appendlist::<MatchOption>::new();
        let all_opts_read = Arc::new(AtomicBool::new(false));
        let all_read = all_opts_read.clone();

        rayon::spawn(move || {
            while let Some(msg) = chan.recv() {
                writer.append(msg);
            }

            all_read.store(true, Ordering::Release);
        });

        Matcher {
            reader,
            all_opts_read,
            previous: Arc::new(AtomicBool::new(false)),
            strategy,
        }
    }

    /// Match the candidates with the pattern. Returns a receiver where the results
    /// can be read from in chunks.
    /// Dropping the receiver stops the matching process.
    pub fn do_match(&mut self, pattern: &str) -> MatchReceiver {
        // Cancel possible previous search
        self.previous.store(true, Ordering::Release);
        self.previous = Arc::new(AtomicBool::new(false));

        // Batch candidates to 512 sized blocks
        // Send each block to an executor
        // Get the results and send to receiver
        let (out, rx) = channel::<Choice>(Self::CHANNEL_SIZE);
        let reader = self.reader.clone();
        let all_opts_read = self.all_opts_read.clone();
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
        let mut available = reader.len();
        let mut taken = 0;
        let stop = self.previous.clone();

        rayon::spawn(move || loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }

            let all_read = all_opts_read.load(Ordering::Relaxed);
            // If we are done reading all available options
            if all_read && available <= taken {
                break;
            }

            if available >= taken + Self::BATCH_SIZE || all_read {
                let size = min(available - taken, Self::BATCH_SIZE);
                let batch = taken..taken + size;
                taken += size;

                let out = out.clone();
                let stop = stop.clone();
                let reader = reader.clone();
                let patterns = patterns.clone();

                rayon::spawn(move || {
                    if stop.load(Ordering::Relaxed) {
                        return;
                    }

                    let opts = reader.slice(batch);
                    for opt in opts.into_iter() {
                        if let Some(ranges) =
                            matches_with(&opt, &patterns, case_sensitive, match_fn)
                        {
                            let score = score(&opt.to_str_lossy(), &ranges);
                            let desc = opt.description();
                            let bytes = opt.bytes();
                            let mat = Choice::new(bytes, ranges, score, desc);

                            if out.blocking_send(mat).is_err() {
                                stop.store(true, Ordering::Release);
                                return;
                            }
                        }
                    }
                });
            } else {
                // TODO Wait for next batch
                available = reader.len();
            }
        });

        MatchReceiver { receiver: rx }
    }
}

// Score a match
fn score(opt: &str, ranges: &[Range<usize>]) -> u32 {
    // Closest match first
    // Shortest item first
    ranges
        .get(0)
        .map(|f| f.start as u32)
        .unwrap_or(opt.len() as u32)
}

fn with_case_sensitivity(opt: &MatchOption, case_sensitive: bool) -> Cow<str> {
    let string = opt.to_str_lossy();
    if case_sensitive {
        return string;
    }

    let has_upper = string.chars().any(|ch| ch.is_ascii_uppercase());
    if !has_upper {
        return string;
    }

    Cow::from(string.to_ascii_lowercase())
}

fn matches_with(
    opt: &MatchOption,
    patterns: &Arc<Vec<String>>,
    case_sensitive: bool,
    f: fn(&str, &str) -> Option<Range<usize>>,
) -> Option<Vec<Range<usize>>> {
    let string: Cow<str> = with_case_sensitivity(opt, case_sensitive);
    let mut matches = vec![];
    for pattern in patterns.iter() {
        // Calculate match and apply offset
        let mut range = (f)(&string, pattern)?;
        range.start += opt.offset();
        range.end += opt.offset();

        matches.push(range);
    }

    Some(matches)
}
