use std::{any::Any, sync::Arc};

use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    actions::jobs::{match_options, MatchedOptions, CHANNEL_SIZE},
    common::matcher::Match,
    editor::{job_broker::KeepInTouch, windows::SelectorOption, Editor},
    job_runner::{Job, JobContext, JobResult},
    server::ClientId,
};

enum MatcherMessage {
    Init(Sender<String>),
    Progress(MatchedOptions),
}

#[derive(Debug, Clone)]
pub(crate) struct StaticMatcher {
    client_id: ClientId,
    opts: Arc<Vec<String>>,
    formatter: Arc<fn(&Editor, ClientId, Match) -> SelectorOption>,
}

impl StaticMatcher {
    pub fn new(
        cid: ClientId,
        opts: Vec<String>,
        f: fn(&Editor, ClientId, Match) -> SelectorOption,
    ) -> StaticMatcher {
        StaticMatcher {
            client_id: cid,
            opts: Arc::new(opts),
            formatter: Arc::new(f),
        }
    }

    pub fn new_default(cid: ClientId, opts: Vec<String>) -> StaticMatcher {
        StaticMatcher {
            client_id: cid,
            opts: Arc::new(opts),
            formatter: Arc::new(|_, _, m| SelectorOption::from(m)),
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
                match_options(orecv, trecv, msend),
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
        let draw = editor.draw_state(self.client_id);
        draw.no_redraw_window();

        if let Ok(msg) = msg.downcast::<MatcherMessage>() {
            let (win, _buf) = editor.win_buf_mut(self.client_id);
            use MatcherMessage::*;
            match *msg {
                Init(sender) => {
                    win.prompt.set_on_input(move |editor, id, input| {
                        let _ = sender.blocking_send(input.to_string());
                    });
                    win.prompt.clear_options();
                }
                Progress(opts) => match opts {
                    MatchedOptions::ClearAll => win.prompt.clear_options(),
                    MatchedOptions::Options(opts) => {
                        let opts: Vec<SelectorOption> = opts
                            .into_iter()
                            .map(|mat| (self.formatter)(editor, self.client_id, mat))
                            .collect();
                        let (win, _buf) = editor.win_buf_mut(self.client_id);
                        win.prompt.provide_options(opts.into());
                    }
                },
            }
        }
    }

    fn on_success(&self, editor: &mut Editor) {}

    fn on_failure(&self, editor: &mut Editor, reason: &str) {}
}
