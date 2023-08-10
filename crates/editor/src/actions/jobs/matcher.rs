mod candidates;

use std::sync::{
    atomic::AtomicUsize,
    mpsc::{Receiver, Sender},
    Arc,
};

use parking_lot::RwLock;

pub(crate) type Candidate = String;

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
    // reader: Candidates,
}

impl Matcher {
    pub const BATCH_SIZE: usize = 512;

    // Create a new matcher.
    pub fn new(chan: Receiver<CandidateMessage>) -> Matcher {
        let candidates = Arc::new(RwLock::new(vec![]));

        rayon::spawn(move || {
            let candidates = candidates.clone();

            while let Ok(msg) = chan.recv() {
                match msg {
                    CandidateMessage::Many(cans) => {
                        let candidates = candidates.write();
                        for can in cans.into_iter() {
                            candidates.push(*can);
                        }
                    }
                    CandidateMessage::One(can) => {
                        let candidates = candidates.write();
                        candidates.push(can);
                    }
                }
            }
        });

        Matcher {}
    }

    /// Match the candidates with the term. Returns a receiver where the results
    /// can be read from.
    /// Dropping the receiver stops the matching process.
    pub fn do_match(&mut self, term: String) -> Receiver<String> {}
}
