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

pub(crate) use matches::*;
pub(crate) use receiver::*;
pub(crate) use strategy::*;

/// Matches options to a term
pub(crate) struct Matcher {
    reader: Reader<String>,
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
        T: MatchOptionReceiver<String> + Send + 'static,
    {
        let (reader, writer) = Appendlist::<String>::new();
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

    /// Match the candidates with the term. Returns a receiver where the results
    /// can be read from in chunks.
    /// Dropping the receiver stops the matching process.
    pub fn do_match(&mut self, term: &str) -> MatchReceiver {
        // Cancel possible previous search
        self.previous.store(true, Ordering::Release);
        self.previous = Arc::new(AtomicBool::new(false));

        // Batch candidates to 512 sized blocks
        // Send each block to an executor
        // Get the results and send to receiver
        let (out, rx) = channel::<Match>(Self::CHANNEL_SIZE);
        let reader = self.reader.clone();
        let all_opts_read = self.all_opts_read.clone();
        let case_sensitive = term.chars().any(|ch| ch.is_uppercase());
        let strat = self.strategy;

        // Apply strategy to term
        // Split term by whitespace and use the resulting terms as independent
        // searches
        let terms: Arc<Vec<String>> = {
            if strat.split() {
                let terms = term.split_whitespace().map(String::from).collect();
                Arc::new(terms)
            } else {
                Arc::new(vec![term.into()])
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
                let terms = terms.clone();

                rayon::spawn(move || {
                    if stop.load(Ordering::Relaxed) {
                        return;
                    }

                    let candidates = reader.slice(batch);
                    for can in candidates.into_iter() {
                        if let Some(ranges) =
                            matches_with(&can, &terms, case_sensitive, strat.get())
                        {
                            let mat = Match {
                                score: score(can.as_str(), &ranges),
                                value: can.clone(),
                                ranges,
                            };

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

fn matches_with(
    string: &str,
    terms: &Arc<Vec<String>>,
    case_sensitive: bool,
    f: fn(&str, &str) -> Option<Range<usize>>,
) -> Option<Vec<Range<usize>>> {
    let string: Cow<str> = if !case_sensitive {
        // TODO unicode casefolding?
        Cow::from(string.to_ascii_lowercase())
    } else {
        Cow::from(string)
    };

    let mut matches = vec![];
    for term in terms.iter() {
        let range = (f)(&string, term)?;
        matches.push(range);
    }

    Some(matches)
}
