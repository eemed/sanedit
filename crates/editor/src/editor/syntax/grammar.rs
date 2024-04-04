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

    pub fn parse(&self, slice: &PieceTreeSlice) -> Result<AST, sanedit_parser::ParseError> {
        // TODO impl
        let content = String::from(slice);
        self.parser.parse(content.as_str())
    }
}

// struct PTCharIter<'a> {
//     slice: PieceTreeSlice<'a>,
// }

// impl<'a> CharReader for PTCharIter<'a> {
//     type I;

//     type O;

//     fn len(&self) -> usize {
//         todo!()
//     }

//     fn stop(&self) -> bool {
//         todo!()
//     }

//     fn chars_rev(&self) -> Self::I {
//         todo!()
//     }

//     fn chars_at(&self, at: usize) -> Self::O {
//         todo!()
//     }
// }
