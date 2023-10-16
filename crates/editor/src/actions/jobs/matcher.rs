use std::mem;

use tokio::sync::mpsc::{Receiver, Sender};

use crate::common::matcher::{Match, MatchReceiver, Matcher};

#[derive(Debug)]
pub(crate) enum MatchedOptions {
    ClearAll,
    Options(Vec<Match>),
}

/// Reads options and filter term from channels and send results to progress
pub(crate) async fn match_options(
    orecv: Receiver<String>,
    mut trecv: Receiver<String>,
    msend: Sender<MatchedOptions>,
) {
    fn spawn(
        msend: Sender<MatchedOptions>,
        mut recv: MatchReceiver,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            const MAX_SIZE: usize = 256;
            let mut matches = Vec::with_capacity(MAX_SIZE);

            while let Some(res) = recv.recv().await {
                matches.push(res);

                if matches.len() >= MAX_SIZE {
                    let opts = mem::take(&mut matches);
                    if let Err(_) = msend.send(MatchedOptions::Options(opts)).await {
                        break;
                    }
                }
            }

            let _ = msend.send(MatchedOptions::Options(matches)).await;
        })
    }

    let mut matcher = Matcher::new(orecv);
    let recv = matcher.do_match("");
    let mut join = spawn(msend.clone(), recv);

    while let Some(term) = trecv.recv().await {
        join.abort();
        let _ = join.await;

        if let Err(_e) = msend.send(MatchedOptions::ClearAll).await {
            break;
        }

        let recv = matcher.do_match(&term);
        join = spawn(msend.clone(), recv);
    }
}
