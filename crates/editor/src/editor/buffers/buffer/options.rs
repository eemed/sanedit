use serde::Deserialize;

use sanedit_buffer::utf8::EndOfLine;

use crate::common::indent::Indent;

// Trick to use with serde
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(remote = "EndOfLine")]
enum EndOfLineDef {
    LF,
    VT,
    FF,
    CR,
    CRLF,
    NEL,
    LS,
    PS,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Options {
    #[serde(with = "EndOfLineDef")]
    pub(crate) eol: EndOfLine,

    /// How many spaces for a tab character, note that tab does not always have
    /// width `tab_width` because tabs are used to align stuff, so it
    /// is "elastic" calculated tabstop - (col % tabstop).
    pub(crate) tabstop: u8,

    // How to indent stuff
    #[serde(flatten)]
    pub(crate) indent: Indent,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            eol: EndOfLine::default(),
            tabstop: 8,
            indent: Indent::default(),
        }
    }
}
