use std::{any::Any, ops::Range};

use pest::{error::InputLocation, Parser};
use sanedit_buffer::ReadOnlyPieceTree;

use crate::{
    editor::{job_broker::KeepInTouch, Editor},
    grammars::{
        self,
        json::{JsonParser, Rule},
    },
    server::{ClientId, Job, JobContext, JobResult},
};

#[derive(Clone)]
pub(crate) struct SyntaxHighlighter {
    client_id: ClientId,
    ropt: ReadOnlyPieceTree,
    range: Range<usize>,
}

impl SyntaxHighlighter {
    pub fn new(id: ClientId, ropt: ReadOnlyPieceTree, range: Range<usize>) -> Self {
        SyntaxHighlighter {
            client_id: id,
            ropt,
            range,
        }
    }
}

impl Job for SyntaxHighlighter {
    fn run(&self, ctx: JobContext) -> JobResult {
        let pt = self.ropt.clone();
        let range = self.range.clone();

        let fut = async move {
            let slice = pt.slice(range);
            let content = String::from(&slice);
            let mut start = 0;

            match JsonParser::parse(Rule::value, &content[start..]) {
                Ok(pairs) => {
                    pairs.flatten().for_each(|pair| {
                        log::info!("Rule:    {:?}", pair.as_rule());
                        log::info!("Span:    {:?}", pair.as_span());
                    });
                    // pairs.tokens().for_each(|tok| {
                    //     log::info!("Token: {tok:?}");
                    // });
                    // for pair in pairs {
                    //     // A pair is a combination of the rule which matched and a span of input
                    //     log::info!("Rule:    {:?}", pair.as_rule());
                    //     log::info!("Span:    {:?}", pair.as_span());
                    //     // log::info!("Text:    {}", pair.as_str());

                    //     // A pair can be converted to an iterator of the tokens which make it up:
                    //     for inner_pair in pair.into_inner() {
                    //         log::info!("Rule:    {:?}", inner_pair.as_rule());
                    //         log::info!("Span:    {:?}", inner_pair.as_span());
                    //     }
                    // }
                }
                Err(e) => {
                    // let at = match e.location {
                    //     InputLocation::Pos(start) => start + 1,
                    //     InputLocation::Span((start, end)) => end,
                    // };
                    // start = at;
                    log::info!("parsing failed: {e}");

                    // if at >= content.len() {
                    //     break;
                    // }
                }
            }

            // let (msend, mrecv) = channel::<Vec<Range<usize>>>(CHANNEL_SIZE);
            // tokio::join!(
            //     Self::search_impl(msend, dir, term, pt, range),
            //     Self::send_matches(ctx, mrecv),
            // );
            Ok(())
        };

        Box::pin(fut)
    }

    fn box_clone(&self) -> crate::server::BoxedJob {
        Box::new((*self).clone())
    }
}

impl KeepInTouch for SyntaxHighlighter {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>) {}

    fn on_success(&self, editor: &mut Editor) {}

    fn on_failure(&self, editor: &mut Editor, reason: &str) {}
}
