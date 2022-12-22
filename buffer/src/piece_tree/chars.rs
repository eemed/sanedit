use super::PieceTree;

#[derive(Debug, Clone)]
pub struct Chars<'a> {
    pt: &'a PieceTree,
}

impl<'a> Chars<'a> {
    pub fn new() -> Chars<'a> {
        // use similar to graphemes or is this better?
        // bstr::decode_utf8();
        todo!()
    }
}
