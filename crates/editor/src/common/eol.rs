use sanedit_buffer::PieceTreeSlice;
use strum::IntoEnumIterator;
use strum_macros::{AsRefStr, EnumIter};

#[derive(Debug, Copy, Clone, PartialEq, EnumIter, AsRefStr)]
pub(crate) enum EOL {
    LF,
    CRLF,
    // TODO add others
}

impl EOL {
    #[cfg(target_os = "windows")]
    const DEFAULT_EOL: EOL = EOL::CRLF;

    #[cfg(not(target_os = "windows"))]
    const DEFAULT_EOL: EOL = EOL::LF;

    pub fn as_str(&self) -> &str {
        match self {
            EOL::LF => "\n",
            EOL::CRLF => "\r\n",
        }
    }

    pub fn is_eol(string: &PieceTreeSlice) -> bool {
        EOL::iter().any(|eol| string == &eol.as_str())
    }

    pub fn is_eol_bytes<B: AsRef<[u8]>>(bytes: B) -> bool {
        EOL::iter().any(|eol| bytes.as_ref() == eol.as_str().as_bytes())
    }
}

impl Default for EOL {
    fn default() -> Self {
        Self::DEFAULT_EOL
    }
}
