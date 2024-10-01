use std::{fmt, mem, sync::Arc, time::Duration};

use sanedit_core::Choice;
use tokio::{
    sync::mpsc::{channel, Receiver, Sender},
    time::{timeout, Instant},
};

use crate::{
    actions::jobs::CHANNEL_SIZE,
    common::matcher::{MatchOption, MatchReceiver, MatchStrategy, Matcher},
    editor::{job_broker::KeepInTouch, Editor},
};

use sanedit_server::{BoxFuture, ClientId, Job, JobContext, JobResult, Kill};

#[derive(Debug)]
pub(crate) enum MatcherMessage {
    Init(Sender<String>),
    Progress(MatchedOptions),
}

/// Provides options to match
pub(crate) trait OptionProvider: fmt::Debug + Sync + Send {
    fn provide(&self, sender: Sender<MatchOption>, kill: Kill) -> BoxFuture<'static, ()>;
}

impl OptionProvider for Arc<Vec<String>> {
    fn provide(&self, sender: Sender<MatchOption>, _kill: Kill) -> BoxFuture<'static, ()> {
        let items = self.clone();

        let fut = async move {
            for opt in items.iter() {
                if sender.send(MatchOption::from(opt.as_str())).await.is_err() {
                    break;
                }
            }
        };

        Box::pin(fut)
    }
}

impl OptionProvider for Arc<Vec<MatchOption>> {
    fn provide(&self, sender: Sender<MatchOption>, _kill: Kill) -> BoxFuture<'static, ()> {
        let items = self.clone();

        let fut = async move {
            for opt in items.iter() {
                if sender.send(opt.clone()).await.is_err() {
                    break;
                }
            }
        };

        Box::pin(fut)
    }
}

/// What to do with the matched results
pub(crate) type MatchResultHandler = fn(&mut Editor, ClientId, MatcherMessage);

#[derive(Debug)]
struct Empty;
impl Empty {
    fn none_result_handler(_editor: &mut Editor, _id: ClientId, _msg: MatcherMessage) {}
}
impl OptionProvider for Empty {
    fn provide(&self, _sender: Sender<MatchOption>, _kill: Kill) -> BoxFuture<'static, ()> {
        Box::pin(async {})
    }
}

pub(crate) struct MatcherJobBuilder {
    cid: ClientId,
    opts: Arc<dyn OptionProvider>,
    strat: MatchStrategy,
    result_handler: MatchResultHandler,
    pattern: String,
}

impl MatcherJobBuilder {
    pub fn new(cid: ClientId) -> MatcherJobBuilder {
        MatcherJobBuilder {
            cid,
            opts: Arc::new(Empty),
            strat: MatchStrategy::default(),
            result_handler: Empty::none_result_handler,
            pattern: String::new(),
        }
    }

    pub fn options<O: OptionProvider + 'static>(mut self, o: O) -> Self {
        self.opts = Arc::new(o);
        self
    }

    pub fn strategy(mut self, strat: MatchStrategy) -> Self {
        self.strat = strat;
        self
    }

    pub fn handler(mut self, handler: MatchResultHandler) -> Self {
        self.result_handler = handler;
        self
    }

    /// Search term to use when starting matching
    pub fn search(mut self, pattern: String) -> Self {
        self.pattern = pattern;
        self
    }

    pub fn build(self) -> MatcherJob {
        let MatcherJobBuilder {
            cid,
            opts,
            strat,
            result_handler,
            pattern: search_term,
        } = self;

        MatcherJob {
            cid,
            strat,
            result_handler,
            opts,
            pattern: search_term,
        }
    }
}

/// Matches options provided by OptionProvider
/// against a term using a matching strategy.
///
/// Matches are reported to match result handler.
#[derive(Debug, Clone)]
pub(crate) struct MatcherJob {
    cid: ClientId,

    /// Provides the options to match against
    opts: Arc<dyn OptionProvider>,

    /// Alters the behavior of matcher
    strat: MatchStrategy,

    /// Handles match results
    result_handler: MatchResultHandler,

    /// Initial search term to use
    pattern: String,
}

impl MatcherJob {
    pub fn builder(cid: ClientId) -> MatcherJobBuilder {
        MatcherJobBuilder::new(cid)
    }

    async fn send_matched_options(mut ctx: JobContext, mut mrecv: Receiver<MatchedOptions>) {
        while let Some(msg) = mrecv.recv().await {
            ctx.send(MatcherMessage::Progress(msg));
        }
    }
}

impl Job for MatcherJob {
    fn run(&self, mut ctx: JobContext) -> JobResult {
        let strat = self.strat;
        let opts = self.opts.clone();
        let patt = self.pattern.clone();

        let fut = async move {
            // Term channel
            let (psend, precv) = channel::<String>(CHANNEL_SIZE);
            // Options channel
            let (osend, orecv) = channel::<MatchOption>(CHANNEL_SIZE);
            // Results channel
            let (msend, mrecv) = channel::<MatchedOptions>(CHANNEL_SIZE);

            ctx.send(MatcherMessage::Init(psend));

            tokio::join!(
                opts.provide(osend, ctx.kill.clone()),
                match_options(orecv, precv, msend, strat, patt),
                Self::send_matched_options(ctx, mrecv),
            );

            Ok(())
        };

        Box::pin(fut)
    }
}

impl KeepInTouch for MatcherJob {
    fn client_id(&self) -> ClientId {
        self.cid
    }

    fn on_message(&self, editor: &mut Editor, msg: Box<dyn std::any::Any>) {
        if let Ok(msg) = msg.downcast::<MatcherMessage>() {
            let id = self.client_id();
            (self.result_handler)(editor, id, *msg);
        }
    }
}

#[derive(Debug)]
pub(crate) enum MatchedOptions {
    Done,
    Options {
        matched: Vec<Choice>,
        clear_old: bool,
    },
}

/// Reads options and filter term from channels and send results to progress
pub(crate) async fn match_options(
    orecv: Receiver<MatchOption>,
    mut precv: Receiver<String>,
    msend: Sender<MatchedOptions>,
    strat: MatchStrategy,
    mut patt: String,
) {
    fn spawn(
        msend: Sender<MatchedOptions>,
        mut recv: MatchReceiver,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            // send matches once we have MAX_SIZE of them.
            const MAX_SIZE: usize = 256;
            let mut matches = Vec::with_capacity(MAX_SIZE);
            let mut clear_old = true;

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

                            if msend
                                .send(MatchedOptions::Options {
                                    matched: opts,
                                    clear_old,
                                })
                                .await
                                .is_err()
                            {
                                break;
                            }
                            clear_old = false;
                        }
                    }
                    Err(_) => {
                        // Timeout
                        // no results for a while, send remaining results
                        last_sent = Instant::now();
                        let opts = mem::take(&mut matches);

                        if !opts.is_empty()
                            && msend
                                .send(MatchedOptions::Options {
                                    matched: opts,
                                    clear_old,
                                })
                                .await
                                .is_err()
                        {
                            break;
                        }
                        clear_old = false;
                    }
                    Ok(None) => break,
                }
            }

            // Clear old in case of no results
            let _ = msend
                .send(MatchedOptions::Options {
                    clear_old,
                    matched: matches,
                })
                .await;
            let _ = msend.send(MatchedOptions::Done).await;
        })
    }

    let mut matcher = Matcher::new(orecv, strat);

    let recv = matcher.do_match(&patt);
    let mut join = spawn(msend.clone(), recv);

    while let Some(t) = precv.recv().await {
        if patt == t {
            continue;
        }
        patt = t;

        join.abort();
        let _ = join.await;

        let recv = matcher.do_match(&patt);
        join = spawn(msend.clone(), recv);
    }
}
