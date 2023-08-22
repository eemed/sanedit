use std::{
    cmp::min,
    mem,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use futures::Future;
use sanedit_utils::appendlist::{Appendlist, Reader};
use tokio::sync::mpsc::{channel, Receiver};

pub(crate) type Candidate = String;

/// Used to provide options to the matcher
#[derive(Debug)]
pub(crate) enum CandidateMessage {
    Many(Box<[Candidate]>),
    One(Candidate),
}

impl From<String> for CandidateMessage {
    fn from(value: String) -> Self {
        CandidateMessage::One(value)
    }
}

/// Trait used to receive candidates using various receiver implementations
pub(crate) trait MatchOptionReceiver<T> {
    fn recv(&mut self) -> Option<T>;
}

/// A matched and scored candidate
#[derive(Debug, Clone)]
pub(crate) struct Match {
    value: String,
    score: u32,
}

impl Match {
    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }

    pub fn score(&self) -> u32 {
        self.score
    }
}

/// Receiver for the match results
#[derive(Debug)]
pub(crate) struct MatchReceiver {
    receiver: Receiver<Match>,
}

impl MatchReceiver {
    pub fn blocking_recv(&mut self) -> Option<Match> {
        self.receiver.blocking_recv()
    }

    pub async fn recv(&mut self) -> Option<Match> {
        self.receiver.recv().await
    }
}

/// Matches options to a term
pub(crate) struct Matcher {
    reader: Reader<Candidate>,
    candidates_done: Arc<AtomicBool>,
}

impl Matcher {
    const BATCH_SIZE: usize = 1024;
    const CHANNEL_SIZE: usize = 1 << 14;

    // Create a new matcher.
    pub fn new<T>(mut chan: T) -> Matcher
    where
        T: MatchOptionReceiver<CandidateMessage> + Send + 'static,
    {
        let (reader, writer) = Appendlist::<Candidate>::new();
        let candidates_done = Arc::new(AtomicBool::new(false));
        let done = candidates_done.clone();

        rayon::spawn(move || {
            while let Some(msg) = chan.recv() {
                match msg {
                    CandidateMessage::Many(mut cans) => {
                        for i in 0..cans.len() {
                            let can = mem::take(&mut cans[i]);
                            writer.append(can);
                        }
                    }
                    CandidateMessage::One(can) => writer.append(can),
                }
            }

            done.store(true, Ordering::Release);
        });

        Matcher {
            reader,
            candidates_done,
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
        let candidates_done = self.candidates_done.clone();
        let term: Arc<String> = Arc::new(term.into());
        let mut available = reader.len();
        let mut taken = 0;
        let stop = Arc::new(AtomicBool::new(false));

        rayon::spawn(move || loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }

            let cdone = candidates_done.load(Ordering::Relaxed);
            if cdone && available <= taken {
                break;
            }

            if available >= taken + Self::BATCH_SIZE || cdone {
                let size = min(available - taken, Self::BATCH_SIZE);
                let batch = taken..taken + size;
                taken += size;

                let out = out.clone();
                let stop = stop.clone();
                let reader = reader.clone();
                let term = term.clone();

                rayon::spawn(move || {
                    if stop.load(Ordering::Relaxed) {
                        return;
                    }

                    let candidates = reader.slice(batch);
                    for can in candidates.into_iter() {
                        if matches_with(&can, &term, false).is_some() {
                            // TODO: scoring algorithm
                            let mat = Match {
                                score: can.len() as u32,
                                value: can.clone(),
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

fn matches_with(string: &str, input: &str, ignore_case: bool) -> Option<usize> {
    if ignore_case {
        string.to_ascii_lowercase().find(input)
    } else {
        string.find(input)
    }
}
