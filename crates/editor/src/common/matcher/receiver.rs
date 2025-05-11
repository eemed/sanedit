use tokio::sync::mpsc::Receiver;

use super::ScoredChoice;

/// Receiver for the match results
#[derive(Debug)]
pub struct MatchReceiver {
    pub(super) receiver: Receiver<ScoredChoice>,
}

impl MatchReceiver {
    pub async fn recv(&mut self) -> Option<ScoredChoice> {
        self.receiver.recv().await
    }
}
