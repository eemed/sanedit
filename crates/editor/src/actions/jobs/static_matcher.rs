use std::{any::Any, sync::Arc};

use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    actions::jobs::{match_options, MatchedOptions, CHANNEL_SIZE},
    common::matcher::{default_match_fn, Match, MatchFn},
    editor::{job_broker::KeepInTouch, windows::SelectorOption, Editor},
    job_runner::{Job, JobContext, JobResult},
    server::ClientId,
};

pub(crate) enum MatcherMessage {
    Init(Sender<String>),
    Progress(MatchedOptions),
}

#[derive(Debug, Clone)]
pub(crate) struct StaticMatcher {
    client_id: ClientId,
    opts: Arc<Vec<String>>,
    on_message: fn(&mut Editor, ClientId, MatcherMessage),
    match_fn: MatchFn,
}

impl StaticMatcher {
    pub fn new(
        cid: ClientId,
        opts: Vec<String>,
        on_message: fn(&mut Editor, ClientId, MatcherMessage),
        match_fn: MatchFn,
    ) -> StaticMatcher {
        StaticMatcher {
            client_id: cid,
            opts: Arc::new(opts),
            on_message,
            match_fn,
        }
    }

    async fn send_options(opts: Arc<Vec<String>>, osend: Sender<String>) {
        for opt in opts.iter() {
            let _ = osend.send(opt.clone()).await;
        }
    }

    async fn send_matched_options(mut ctx: JobContext, mut mrecv: Receiver<MatchedOptions>) {
        while let Some(msg) = mrecv.recv().await {
            ctx.send(MatcherMessage::Progress(msg));
        }
    }
}

impl Job for StaticMatcher {
    fn run(&self, mut ctx: JobContext) -> JobResult {
        let opts = self.opts.clone();

        let fut = async move {
            let (tsend, trecv) = channel::<String>(CHANNEL_SIZE);
            let (osend, orecv) = channel::<String>(CHANNEL_SIZE);
            let (msend, mrecv) = channel::<MatchedOptions>(CHANNEL_SIZE);

            ctx.send(MatcherMessage::Init(tsend));

            tokio::join!(
                Self::send_options(opts, osend),
                match_options(orecv, trecv, msend, default_match_fn),
                Self::send_matched_options(ctx, mrecv),
            );

            Ok(())
        };

        Box::pin(fut)
    }
}

impl KeepInTouch for StaticMatcher {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>) {
        if let Ok(msg) = msg.downcast::<MatcherMessage>() {
            let id = self.client_id();
            (self.on_message)(editor, id, *msg);
        }
    }

    fn on_success(&self, editor: &mut Editor) {}

    fn on_failure(&self, editor: &mut Editor, reason: &str) {}
}
