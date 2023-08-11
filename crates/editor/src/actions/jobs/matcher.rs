mod candidates;

use std::sync::{
    atomic::AtomicUsize,
    mpsc::{Receiver, Sender},
    Arc,
};

use parking_lot::RwLock;

use self::candidates::{Candidate, Candidates, Reader};

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
    reader: Reader,
}

impl Matcher {
    // Create a new matcher.
    pub fn new(chan: Receiver<CandidateMessage>) -> Matcher {
        let (reader, writer) = Candidates::new();

        rayon::spawn(move || {
            while let Ok(msg) = chan.recv() {
                match msg {
                    CandidateMessage::Many(cans) => cans.into_iter().for_each(|can| {
                        writer.append(*can);
                    }),
                    CandidateMessage::One(can) => writer.append(can),
                }
            }
        });

        Matcher { reader }
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
    }
}
