use tokio::sync::mpsc::Receiver;

use super::matches::Match;

/// Trait used to receive candidates using various receiver implementations
pub(crate) trait MatchOptionReceiver<T> {
    fn recv(&mut self) -> Option<T>;
}

impl<T> MatchOptionReceiver<T> for Receiver<T> {
    fn recv(&mut self) -> Option<T> {
        self.blocking_recv()
    }
}

/// Receiver for the match results
#[derive(Debug)]
pub(crate) struct MatchReceiver {
    pub(super) receiver: Receiver<Match>,
}

impl MatchReceiver {
    pub fn blocking_recv(&mut self) -> Option<Match> {
        self.receiver.blocking_recv()
    }

    pub async fn recv(&mut self) -> Option<Match> {
        self.receiver.recv().await
    }
}
