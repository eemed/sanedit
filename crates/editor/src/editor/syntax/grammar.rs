use std::{fs::File, path::Path};

use anyhow::bail;
use sanedit_buffer::PieceTreeSlice;
use sanedit_parser::{PikaParser, AST};

#[derive(Debug)]
pub(crate) struct Grammar {
    parser: PikaParser,
}

impl Grammar {
    pub fn from_path(peg: &Path) -> anyhow::Result<Grammar> {
        let file = match File::open(&peg) {
            Ok(f) => f,
            Err(e) => bail!("Failed to read PEG file: {:?}", peg),
        };

        match PikaParser::new(file) {
            Ok(p) => Ok(Grammar { parser: p }),
            Err(e) => bail!("Failed to create grammar from PEG file: {:?}: {e}", peg),
        }
    }

    pub fn parse(&self, slice: &PieceTreeSlice) -> AST {
        let content = String::from(slice);
        self.parser.parse(&content)
    }
}
