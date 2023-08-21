use std::mem;

use tokio::sync::mpsc::Receiver;

use crate::{
    actions::jobs::MatcherResult,
    common::matcher::{CandidateMessage, Match, Matcher, MatcherReceiver},
    server::{JobProgress, JobProgressSender},
};

impl<T> MatcherReceiver<T> for Receiver<T> {
    fn recv(&mut self) -> Option<T> {
        self.blocking_recv()
    }
}

/// Reads options and filter term from channels and send good results to
/// progress
pub(crate) async fn matcher(
    mut out: JobProgressSender,
    opt_in: Receiver<CandidateMessage>,
    mut term_in: Receiver<String>,
) -> bool {
    fn spawn(
        mut out: JobProgressSender,
        mut rx: Receiver<Vec<Match>>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(mut res) = rx.recv().await {
                if let Err(_e) = out
                    .send(JobProgress::Output(Box::new(MatcherResult::Matches(
                        mem::take(&mut res),
                    ))))
                    .await
                {
                    break;
                }
            }
        })
    }

    let mut matcher = Matcher::new(opt_in);
    let rx = matcher.do_match("");
    let mut recv = spawn(out.clone(), rx);

    while let Some(term) = term_in.recv().await {
        recv.abort();
        let _ = recv.await;

        if let Err(_e) = out
            .send(JobProgress::Output(Box::new(MatcherResult::Reset)))
            .await
        {
            break;
        }

        let rx = matcher.do_match(&term);
        recv = spawn(out.clone(), rx);
    }

    true
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
