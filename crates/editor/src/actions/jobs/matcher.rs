use std::{
    cmp::min,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
};

use parking_lot::RwLock;
use sanedit_utils::appendlist::{Appendlist, Reader};

type Candidate = String;

pub(crate) enum CandidateMessage {
    Many(Box<[Candidate]>),
    One(Candidate),
}

impl From<String> for CandidateMessage {
    fn from(value: String) -> Self {
        CandidateMessage::One(value)
    }
}

pub(crate) struct Matcher {
    reader: Reader<Candidate>,
    candidates_done: Arc<AtomicBool>,
}

impl Matcher {
    const BATCH_SIZE: usize = 512;

    // Create a new matcher.
    pub fn new(chan: Receiver<CandidateMessage>) -> Matcher {
        let (reader, writer) = Appendlist::<Candidate>::new();
        let candidates_done = Arc::new(AtomicBool::new(false));
        let done = candidates_done.clone();

        rayon::spawn(move || {
            while let Ok(msg) = chan.recv() {
                match msg {
                    CandidateMessage::Many(cans) => cans.into_iter().for_each(|can| {
                        writer.append(can.clone());
                    }),
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
    /// can be read from.
    /// Dropping the receiver stops the matching process.
    pub fn do_match(&mut self, term: String) -> Receiver<String> {
        // TODO: what to do if more candidates are still coming? wait and block?
        //
        // Batch candidates to 512 sized blocks
        // Send each block to an executor
        // Get the results and send to receiver

        let (out, rx) = mpsc::channel::<String>();
        let reader = self.reader.clone();
        let candidates_done = self.candidates_done.clone();
        let term = Arc::new(term);
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
                    for (i, can) in candidates.into_iter().enumerate() {
                        if matches_with(&can, &term, false).is_some() {
                            if out.send(can.clone()).is_err() {
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

        rx
    }
}

fn matches_with(string: &str, input: &str, ignore_case: bool) -> Option<usize> {
    if ignore_case {
        string.to_ascii_lowercase().find(input)
    } else {
        string.find(input)
    }
}

#[cfg(test)]
mod test {
    use std::thread;

    use super::*;

    #[test]
    fn matcher() {
        let (tx, rx) = mpsc::channel();
        let join = thread::spawn(move || {
            let mut i = 1;
            while i < 1000 {
                tx.send(CandidateMessage::One(format!("Message {i}")));
                i += 1;
            }
        });
        let mut matcher = Matcher::new(rx);
        let mut result = matcher.do_match("".into());
        while let Ok(res) = result.recv() {
            println!("Received: {res}");
        }
    }
}
