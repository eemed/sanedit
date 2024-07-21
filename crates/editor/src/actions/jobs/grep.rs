use std::any::Any;
use std::error::Error;
use std::ffi::OsStr;
use std::ops::Range;
use std::os::unix::prelude::OsStrExt;
use std::path::{Path, PathBuf};

use grep::matcher::{LineTerminator, Matcher};
use grep::regex::{RegexMatcher, RegexMatcherBuilder};
use grep::searcher::{BinaryDetection, Searcher, SearcherBuilder, Sink, SinkMatch};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::actions::jobs::{OptionProvider, CHANNEL_SIZE};
use crate::common::matcher::MatchOption;
use crate::{
    editor::{job_broker::KeepInTouch, Editor},
    job_runner::{Job, JobContext, JobResult},
    server::ClientId,
};

use super::FileOptionProvider;

#[derive(Clone)]
pub(crate) struct Grep {
    client_id: ClientId,
    pattern: String,
    path: PathBuf,
    file_opt_provider: FileOptionProvider,
}

impl Grep {
    pub fn new(pattern: &str, path: &Path, ignore: &[String], id: ClientId) -> Grep {
        let fprovider = FileOptionProvider::new(path, ignore);

        Grep {
            client_id: id,
            pattern: pattern.into(),
            path: path.to_path_buf(),
            file_opt_provider: fprovider,
        }
    }

    async fn grep(mut orecv: Receiver<MatchOption>, pattern: &str, msend: Sender<GrepMessage>) {
        let searcher = SearcherBuilder::new()
            .binary_detection(BinaryDetection::quit(b'\x00'))
            .line_terminator(LineTerminator::byte(b'\n'))
            .line_number(true)
            .multi_line(false)
            .build();

        let matcher = RegexMatcherBuilder::new()
            .line_terminator(Some(b'\n'))
            .case_insensitive(false)
            .case_smart(false)
            .word(false)
            .build(&pattern)
            .expect("Cannot build RegexMatcher");

        while let Some(opt) = orecv.recv().await {
            // TODO if we have unsaved buffer grep that instead.

            let mut searcher = searcher.clone();
            let matcher = matcher.clone();
            let msend = msend.clone();

            rayon::spawn(move || {
                let ospath = OsStr::from_bytes(&opt.value);
                let path = PathBuf::from(ospath);
                let rsend = ResultSender {
                    matcher: &matcher,
                    sender: msend,
                    path: &path,
                    matches: vec![],
                };
                let _ = searcher.search_path(&matcher, &path, rsend);
            });
        }
    }
}

impl Job for Grep {
    fn run(&self, ctx: JobContext) -> JobResult {
        let fopts = self.file_opt_provider.clone();
        let pattern = self.pattern.clone();

        let fut = async move {
            // Options channel
            let (osend, orecv) = channel::<MatchOption>(CHANNEL_SIZE);
            // Results channel
            let (msend, mrecv) = channel::<GrepMessage>(CHANNEL_SIZE);

            tokio::join!(
                fopts.provide(osend, ctx.kill.clone()),
                Self::grep(orecv, &pattern, msend),
            );

            Ok(())
        };

        Box::pin(fut)
    }
}

impl KeepInTouch for Grep {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>) {
        if let Ok(msg) = msg.downcast::<GrepMessage>() {
            todo!("handle grep results")
        }
    }
}

#[derive(Debug)]
struct GrepMatch {
    text: String,
    matches: Vec<Range<usize>>,
}

enum GrepMessage {
    Result {
        path: PathBuf,
        matches: Vec<GrepMatch>,
    },
}

#[derive(Debug)]
struct ResultSender<'a> {
    matcher: &'a RegexMatcher,
    sender: Sender<GrepMessage>,
    path: &'a Path,
    matches: Vec<GrepMatch>,
}

impl<'a> Sink for ResultSender<'a> {
    type Error = Box<dyn Error>;

    fn matched(&mut self, _searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        let Ok(text) = std::str::from_utf8(mat.bytes()) else { return Ok(true); };

        let mut matches = vec![];
        self.matcher
            .find_iter(mat.bytes(), |m| {
                matches.push(m.start()..m.end());
                true
            })
            .ok();

        if !matches.is_empty() {
            let gmat = GrepMatch {
                text: text.to_string(),
                matches,
            };

            self.matches.push(gmat);
        }

        Ok(true)
    }

    fn finish(
        &mut self,
        _searcher: &Searcher,
        _: &grep::searcher::SinkFinish,
    ) -> Result<(), Self::Error> {
        let matches = std::mem::take(&mut self.matches);
        if !matches.is_empty() {
            let _ = self.sender.blocking_send(GrepMessage::Result {
                path: self.path.to_path_buf(),
                matches,
            });
        }

        Ok(())
    }
}
