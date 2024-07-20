use std::any::Any;
use std::path::{Path, PathBuf};

use grep::matcher::LineTerminator;
use grep::regex::{RegexMatcher, RegexMatcherBuilder};
use grep::searcher::{BinaryDetection, Searcher, SearcherBuilder};

use crate::{
    editor::{job_broker::KeepInTouch, Editor},
    job_runner::{Job, JobContext, JobResult},
    server::ClientId,
};

#[derive(Clone)]
pub(crate) struct Grep {
    client_id: ClientId,
    pattern: String,
    path: PathBuf,
    searcher: Searcher,
    matcher: RegexMatcher,
}

impl Grep {
    pub fn new(pattern: &str, path: &Path, id: ClientId) -> Grep {
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

        Grep {
            client_id: id,
            pattern: pattern.into(),
            path: path.to_path_buf(),
            searcher,
            matcher,
        }
    }
}

impl Job for Grep {
    fn run(&self, ctx: JobContext) -> JobResult {
        let fut = async move { Ok(()) };

        Box::pin(fut)
    }
}

impl KeepInTouch for Grep {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>) {}

    fn on_success(&self, editor: &mut Editor) {}

    fn on_failure(&self, editor: &mut Editor, reason: &str) {}
}
