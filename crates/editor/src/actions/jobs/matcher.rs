use std::{
    fmt,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use sanedit_utils::{
    appendlist::{Appendlist, Reader, Writer},
    sorted_vec::SortedVec,
};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    actions::jobs::CHANNEL_SIZE,
    common::matcher::{Choice, MatchStrategy, Matcher, ScoredChoice},
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
    fn provide(
        &self,
        sender: Writer<Arc<Choice>>,
        kill: Kill,
        done: Arc<AtomicUsize>,
    ) -> BoxFuture<'static, ()>;
}

impl OptionProvider for Arc<Vec<String>> {
    fn provide(
        &self,
        sender: Writer<Arc<Choice>>,
        _kill: Kill,
        done: Arc<AtomicUsize>,
    ) -> BoxFuture<'static, ()> {
        let items = self.clone();

        let fut = async move {
            let mut n = 0;
            for opt in items.iter() {
                n += 1;
                sender.append(Choice::from_text(opt.clone()));
            }

            done.store(n, Ordering::Release);
        };

        Box::pin(fut)
    }
}

impl OptionProvider for Arc<Vec<Arc<Choice>>> {
    fn provide(
        &self,
        sender: Writer<Arc<Choice>>,
        _kill: Kill,
        done: Arc<AtomicUsize>,
    ) -> BoxFuture<'static, ()> {
        let items = self.clone();

        let fut = async move {
            let mut n = 0;
            for opt in items.iter() {
                n += 1;
                sender.append(opt.clone());
            }

            done.store(n, Ordering::Release);
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
    fn provide(
        &self,
        _sender: Writer<Arc<Choice>>,
        _kill: Kill,
        done: Arc<AtomicUsize>,
    ) -> BoxFuture<'static, ()> {
        Box::pin(async move {
            done.store(0, Ordering::Release);
        })
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
}

impl Job for MatcherJob {
    fn run(&self, mut ctx: JobContext) -> JobResult {
        let strat = self.strat;
        let opts = self.opts.clone();
        let mut patt = self.pattern.clone();

        let fut = async move {
            // Term channel
            let (psend, mut precv) = channel::<String>(CHANNEL_SIZE);
            let (reader, writer) = Appendlist::<Arc<Choice>>::new();

            ctx.send(MatcherMessage::Init(psend));

            let kill = ctx.kill.clone();
            let mut matcher = Matcher::new(reader, strat, ctx);
            let read_done = matcher.read_done();

            tokio::join!(opts.provide(writer, kill, read_done), async {
                // Start matching
                matcher.do_match(&patt);

                while let Some(t) = precv.recv().await {
                    if patt == t {
                        continue;
                    }
                    patt = t;
                    matcher.do_match(&patt);
                }
            });

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
        matched: SortedVec<ScoredChoice>,
        clear_old: bool,
    },
}
