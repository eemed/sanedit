use std::{fs::File, path::Path};

use anyhow::bail;
use sanedit_buffer::{Bytes, PieceTreeSlice};
use sanedit_parser::{Annotation, ByteReader, CaptureID, CaptureList, Parser};
use tokio::sync::broadcast;

#[derive(Debug)]
pub(crate) struct Grammar {
    parser: Parser,
}

impl Grammar {
    pub fn from_path(peg: &Path) -> anyhow::Result<Grammar> {
        let file = match File::open(&peg) {
            Ok(f) => f,
            Err(e) => bail!("Failed to read PEG file: {:?}", peg),
        };

        match Parser::new(file) {
            Ok(p) => Ok(Grammar { parser: p }),
            Err(e) => bail!("Failed to create grammar from PEG file: {:?}: {e}", peg),
        }
    }

    pub fn label_for(&self, id: CaptureID) -> &str {
        self.parser.label_for(id)
    }

    pub fn annotations_for(&self, id: CaptureID) -> &[Annotation] {
        self.parser.annotations_for(id)
    }

    pub fn parse(
        &self,
        slice: &PieceTreeSlice,
        kill: broadcast::Receiver<()>,
    ) -> Result<CaptureList, sanedit_parser::ParseError> {
        let reader = PTReader {
            pt: slice.clone(),
            kill,
        };
        self.parser.parse(reader)
    }
}

struct PTIter<'a>(Bytes<'a>);
impl<'a> Iterator for PTIter<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

// TODO optimize, check performance using a bytes iterator
// and just cloning it, and limiting to a range
struct PTReader<'a> {
    pt: PieceTreeSlice<'a>,
    kill: broadcast::Receiver<()>,
}

impl<'a> ByteReader for PTReader<'a> {
    type I = PTIter<'a>;

    fn len(&self) -> u64 {
        self.pt.len()
    }

    fn stop(&self) -> bool {
        !self.kill.is_empty()
    }

    fn iter(&self, range: std::ops::Range<u64>) -> Self::I {
        let slice = self.pt.slice(range);
        let bytes = slice.bytes();
        PTIter(bytes)
    }
}
