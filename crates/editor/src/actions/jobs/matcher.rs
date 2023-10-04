use std::any::Any;

use crate::{
    common::matcher::Matcher,
    editor::{job_broker::KeepInTouch, Editor},
    server::{BoxedJob, Job, JobContext, JobResult},
};

// use std::{mem, ops::Index};

// use tokio::sync::mpsc::Receiver;

// use crate::{
//     common::matcher::{CandidateMessage, Match, MatchOptionReceiver, MatchReceiver, Matcher},
//     server::{JobProgress, JobProgressSender},
// };

// impl<T> MatchOptionReceiver<T> for Receiver<T> {
//     fn recv(&mut self) -> Option<T> {
//         self.blocking_recv()
//     }
// }

// /// Matcher result the job returns
// #[derive(Debug)]
// pub(crate) enum MatcherResult {
//     Reset,
//     Matches(Matches),
// }

// /// Vector of matches sorted by score
// #[derive(Default, Debug)]
// pub(crate) struct Matches {
//     inner: Vec<Match>,
// }

// impl Matches {
//     pub fn len(&self) -> usize {
//         self.inner.len()
//     }
// }

// impl Index<usize> for Matches {
//     type Output = Match;

//     fn index(&self, index: usize) -> &Self::Output {
//         &self.inner[index]
//     }
// }

// impl From<Vec<Match>> for Matches {
//     fn from(mut matches: Vec<Match>) -> Self {
//         matches.sort_by(|a, b| a.score().cmp(&b.score()));
//         Matches { inner: matches }
//     }
// }

// /// Reads options and filter term from channels and send results to progress
// pub(crate) async fn matcher(
//     mut out: JobProgressSender,
//     opt_in: Receiver<CandidateMessage>,
//     mut term_in: Receiver<String>,
// ) -> bool {
//     fn spawn(mut out: JobProgressSender, mut rx: MatchReceiver) -> tokio::task::JoinHandle<()> {
//         async fn send(out: &mut JobProgressSender, matches: &mut Vec<Match>) -> bool {
//             let res = out
//                 .send(JobProgress::Output(Box::new(MatcherResult::Matches(
//                     mem::take(matches).into(),
//                 ))))
//                 .await;
//             res.is_ok()
//         }

//         tokio::spawn(async move {
//             const MAX_SIZE: usize = 256;
//             let mut matches = Vec::with_capacity(MAX_SIZE);

//             while let Some(res) = rx.recv().await {
//                 matches.push(res);

//                 if matches.len() >= MAX_SIZE {
//                     if !send(&mut out, &mut matches).await {
//                         break;
//                     }
//                 }
//             }

//             send(&mut out, &mut matches).await;
//         })
//     }

//     let mut matcher = Matcher::new(opt_in);
//     let rx = matcher.do_match("");
//     let mut recv = spawn(out.clone(), rx);

//     while let Some(term) = term_in.recv().await {
//         recv.abort();
//         let _ = recv.await;

//         if let Err(_e) = out
//             .send(JobProgress::Output(Box::new(MatcherResult::Reset)))
//             .await
//         {
//             break;
//         }

//         let rx = matcher.do_match(&term);
//         recv = spawn(out.clone(), rx);
//     }

//     true
// }

// #[cfg(test)]
// mod test {
//     use std::thread;

//     use super::*;

//     #[test]
//     fn matcher() {
//         let (tx, rx) = mpsc::channel();
//         let join = thread::spawn(move || {
//             let mut i = 1;
//             while i < 1000 {
//                 tx.send(CandidateMessage::One(format!("Message {i}")));
//                 i += 1;
//             }
//         });
//         let mut matcher = Matcher::new(rx);
//         let mut result = matcher.do_match("".into());
//         while let Ok(res) = result.recv() {
//             println!("Received: {res}");
//         }
//     }
// }
