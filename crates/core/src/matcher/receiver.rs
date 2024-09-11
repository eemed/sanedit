use tokio::sync::mpsc::Receiver;

use crate::Choice;

/// Trait used to receive candidates using various receiver implementations
pub trait MatchOptionReceiver<T> {
    fn recv(&mut self) -> Option<T>;
}

impl<T> MatchOptionReceiver<T> for Receiver<T> {
    fn recv(&mut self) -> Option<T> {
        self.blocking_recv()
    }
}

/// Receiver for the match results
#[derive(Debug)]
pub struct MatchReceiver {
    pub(super) receiver: Receiver<Choice>,
}

impl MatchReceiver {
    pub fn blocking_recv(&mut self) -> Option<Choice> {
        self.receiver.blocking_recv()
    }

    pub async fn recv(&mut self) -> Option<Choice> {
        self.receiver.recv().await
    }
}
