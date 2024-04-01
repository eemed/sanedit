use std::{mem, time::Duration};

use tokio::{
    sync::mpsc::{Receiver, Sender},
    time::{timeout, Instant},
};

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
            // send matches once we have MAX_SIZE of them.
            const MAX_SIZE: usize = 256;
            let mut matches = Vec::with_capacity(MAX_SIZE);

            // If matches come in slowly (large search) the MAX_SIZE will not be met.
            // Add in a time limit to send any matches
            let limit = Duration::from_millis(1000 / 30); // 30fps
            let mut last_sent = Instant::now();

            loop {
                let result = if matches.is_empty() {
                    let received = recv.recv().await;
                    Ok(received)
                } else {
                    timeout(limit, recv.recv()).await
                };

                match result {
                    Ok(Some(res)) => {
                        matches.push(res);

                        // Check time incase we are dripfed results
                        let now = Instant::now();
                        if matches.len() >= MAX_SIZE || now.duration_since(last_sent) >= limit {
                            last_sent = now;
                            let opts = mem::take(&mut matches);

                            if let Err(_) = msend.send(MatchedOptions::Options(opts)).await {
                                break;
                            }
                        }
                    }
                    Err(_) => {
                        // Timeout
                        // no results for a while, send remaining results
                        last_sent = Instant::now();
                        let opts = mem::take(&mut matches);

                        if let Err(_) = msend.send(MatchedOptions::Options(opts)).await {
                            break;
                        }
                    }
                    Ok(None) => break,
                }
            }

            let _ = msend.send(MatchedOptions::Options(matches)).await;
        })
    }

    let mut matcher = Matcher::new(orecv);
    let mut term = String::new();
    let recv = matcher.do_match(&term);
    let mut join = spawn(msend.clone(), recv);

    while let Some(t) = trecv.recv().await {
        if term == t {
            continue;
        }
        term = t;

        join.abort();
        let _ = join.await;

        if let Err(_e) = msend.send(MatchedOptions::ClearAll).await {
            break;
        }

        let recv = matcher.do_match(&term);
        join = spawn(msend.clone(), recv);
    }
}
