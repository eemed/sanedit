mod matches;
mod receiver;

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

/// Matches options to a term
pub(crate) struct Matcher {
    reader: Reader<String>,
    all_opts_read: Arc<AtomicBool>,
}

impl Matcher {
    const BATCH_SIZE: usize = 1024;
    const CHANNEL_SIZE: usize = 1024;

    // Create a new matcher.
    pub fn new<T>(mut chan: T) -> Matcher
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
        }
    }

    /// Match the candidates with the term. Returns a receiver where the results
    /// can be read from in chunks.
    /// Dropping the receiver stops the matching process.
    pub fn do_match(&mut self, term: &str) -> MatchReceiver {
        // Batch candidates to 512 sized blocks
        // Send each block to an executor
        // Get the results and send to receiver

        let (out, rx) = channel::<Match>(Self::CHANNEL_SIZE);
        let reader = self.reader.clone();
        let all_opts_read = self.all_opts_read.clone();
        let case_sensitive = term.chars().any(|ch| ch.is_uppercase());
        // Split term by whitespace and use the resulting terms as independent
        // searches
        let terms: Vec<String> = term.split_whitespace().map(String::from).collect();
        let terms: Arc<Vec<String>> = Arc::new(terms);
        let mut available = reader.len();
        let mut taken = 0;
        let stop = Arc::new(AtomicBool::new(false));

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
                        if let Some(ranges) = matches_with(&can, &terms, case_sensitive) {
                            // TODO: scoring algorithm
                            let mat = Match {
                                score: can.len() as u32,
                                value: can.clone(),
                                ranges,
                            };

                            if out.blocking_send(mat).is_err() {
                                stop.store(true, Ordering::Release);
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

fn matches_with(
    string: &str,
    terms: &Arc<Vec<String>>,
    case_sensitive: bool,
) -> Option<Vec<Range<usize>>> {
    let string: Cow<str> = if !case_sensitive {
        Cow::from(string.to_lowercase())
    } else {
        Cow::from(string)
    };

    // TODO unicode casefolding
    let mut matches = vec![];
    for term in terms.iter() {
        let start = string.find(term)?;
        matches.push(start..start + term.len());
    }

    Some(matches)
}
