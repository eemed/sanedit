use std::str::Chars;

use crate::server::ClientId;

enum MatcherMessage {
    Init(Sender<String>),
    Progress(MatchedOptions),
}

#[derive(Debug, Clone)]
pub(crate) struct PrefixMatcher {
    client_id: ClientId,
    opts: Arc<TrieNode>,
    formatter: Arc<fn(&Editor, ClientId, Match) -> SelectorOption>,
}

impl PrefixMatcher {
    pub fn new(
        cid: ClientId,
        opts: Vec<String>,
        f: fn(&Editor, ClientId, Match) -> SelectorOption,
    ) -> PrefixMatcher {
        PrefixMatcher {
            client_id: cid,
            opts: Arc::new(opts),
            formatter: Arc::new(f),
        }
    }

    pub fn new_default(cid: ClientId, opts: Vec<String>) -> PrefixMatcher {
        PrefixMatcher {
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

impl Job for PrefixMatcher {
    fn run(&self, mut ctx: JobContext) -> JobResult {
        let opts = self.opts.clone();

        let fut = async move {
            let (tsend, trecv) = channel::<String>(CHANNEL_SIZE);
            let (osend, orecv) = channel::<String>(CHANNEL_SIZE);
            let (msend, mrecv) = channel::<MatchedOptions>(CHANNEL_SIZE);

            ctx.send(MatcherMessage::Init(tsend));

            while let Some(term) = trecv.recv() {}

            Ok(())
        };

        Box::pin(fut)
    }
}

impl KeepInTouch for PrefixMatcher {
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

#[derive(Debug)]
struct TrieNode {
    end: bool,
    children: HashMap<char, TrieNode>,
}

impl TrieNode {
    pub fn get(&self, term: &str) -> Vec<String> {
        let mut chars = term.chars();
        self.get_char(&mut chars)
    }

    fn get_char(&self, chars: &mut Chars) -> Vec<String> {}
}
