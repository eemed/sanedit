use std::ops::Deref;

use strum_macros::{AsRefStr, EnumIter};

#[cfg(target_os = "windows")]
pub(crate) const DEFAULT_EOL: EOL = EOL::CRLF;

#[cfg(not(target_os = "windows"))]
pub(crate) const DEFAULT_EOL: EOL = EOL::LF;

#[derive(Debug, Copy, Clone, PartialEq, EnumIter, AsRefStr)]
pub(crate) enum EOL {
    LF,
    CRLF,
}

impl EOL {
    pub fn as_str(&self) -> &str {
        match self {
            EOL::LF => "\n",
            EOL::CRLF => "\r\n",
        }
    }
}
