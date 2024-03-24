use std::{fs::File, path::Path, time::SystemTime};

use anyhow::bail;
use sanedit_buffer::PieceTreeSlice;
use sanedit_parser::{PikaParser, AST};

use crate::{common::dirs::FILETYPE_DIR, editor::buffers::Filetype};

#[derive(Debug)]
pub(crate) struct Grammar {
    modified: SystemTime,
    parser: PikaParser,
}

impl Grammar {
    pub fn for_filetype(filetype: &Filetype, conf_dir: &Path) -> anyhow::Result<Grammar> {
        let f = Self::grammar_file(filetype, conf_dir)?;
        let metadata = f.metadata()?;
        let modified = metadata.modified()?;

        match PikaParser::new(f) {
            Ok(p) => Ok(Grammar {
                modified,
                parser: p,
            }),
            Err(e) => bail!(
                "Grammar PEG failed to load for filetype {}: {e}",
                filetype.as_str()
            ),
        }
    }

    fn grammar_file(filetype: &Filetype, conf_dir: &Path) -> anyhow::Result<File> {
        let ft = filetype.as_str();
        let peg = {
            let mut conf = conf_dir.to_path_buf();
            conf.push(FILETYPE_DIR);
            conf.push(ft);
            conf.push(format!("{}.peg", ft));
            conf
        };

        match File::open(&peg) {
            Ok(f) => Ok(f),
            Err(e) => bail!("Grammar PEG file error for filetype {ft}: {e}"),
        }
    }

    pub fn filetype_modified_at(
        filetype: &Filetype,
        conf_dir: &Path,
    ) -> anyhow::Result<SystemTime> {
        let f = Self::grammar_file(filetype, conf_dir)?;
        let metadata = f.metadata()?;
        let modified = metadata.modified()?;
        Ok(modified)
    }

    pub fn file_modified_at(&self) -> &SystemTime {
        &self.modified
    }

    pub fn parse(&self, slice: &PieceTreeSlice) -> AST {
        let content = String::from(slice);
        self.parser.parse(&content)
    }
}
