use sanedit_buffer::utf8::EndOfLine;

#[derive(Debug)]
pub(crate) struct Options {
    pub(crate) eol: EndOfLine,
    /// How many spaces for a tab character, note that tab does not always have
    /// width `tab_width` because tabs are used to align stuff, so it
    /// is "elastic" calculated tabstop - (col % tabstop).
    pub(crate) tabstop: u8,

    // if some then when inserting a tab it is replaced by spaces
    // separate from tabstop because we might want to align something with tabs
    pub(crate) tab_spaces: Option<usize>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            eol: EndOfLine::default(),
            tabstop: 8,
            tab_spaces: Some(4),
        }
    }
}
