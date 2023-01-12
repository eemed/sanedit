use strum_macros::{AsRefStr, EnumIter};

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

impl Default for EOL {
    fn default() -> Self {
        #[cfg(target_os = "windows")]
        const DEFAULT_EOL: EOL = EOL::CRLF;

        #[cfg(not(target_os = "windows"))]
        const DEFAULT_EOL: EOL = EOL::LF;

        DEFAULT_EOL
    }
}
