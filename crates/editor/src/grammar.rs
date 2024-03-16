use std::{fs::File, path::Path};

use anyhow::bail;
use sanedit_buffer::PieceTreeSlice;
use sanedit_parser::{PikaParser, AST};

use crate::{common::dirs::FILETYPE_DIR, editor::buffers::Filetype};

#[derive(Debug)]
pub(crate) struct Grammar {
    parser: PikaParser,
}

impl Grammar {
    pub fn for_filetype(filetype: &Filetype, conf_dir: &Path) -> anyhow::Result<Grammar> {
        let ft = filetype.as_str();
        let peg = {
            let mut conf = conf_dir.to_path_buf();
            conf.push(FILETYPE_DIR);
            conf.push(ft);
            conf.push(format!("{}.peg", ft));
            conf
        };

        let f = match File::open(&peg) {
            Ok(f) => f,
            Err(e) => bail!("Grammar PEG file error for filetype {ft}: {e}"),
        };

        match PikaParser::new(f) {
            Ok(p) => Ok(Grammar { parser: p }),
            Err(e) => bail!("Grammar PEG failed to load for filetype {ft}: {e}"),
        }
    }

    pub fn parse(&self, slice: &PieceTreeSlice) -> AST {
        let content = String::from(slice);
        self.parser.parse(&content)
    }
}
