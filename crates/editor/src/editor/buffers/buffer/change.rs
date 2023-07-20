use sanedit_buffer::utf8::EndOfLine;

/// Describes what changed in the buffer when the change was made.
#[derive(Debug, Clone)]
pub(crate) struct Change {
    pub(crate) needs_undo_point: bool,
    pub(crate) kind: ChangeKind,
}

impl Change {
    pub fn new(prev: Option<&ChangeKind>, next: ChangeKind, is_modified: bool) -> Change {
        let needs_undo_point = needs_undo_point(prev, &next, is_modified);
        Change {
            kind: next,
            needs_undo_point,
        }
    }
}

fn needs_undo_point(prev: Option<&ChangeKind>, next: &ChangeKind, is_modified: bool) -> bool {
    use ChangeKind::*;

    if !is_modified || prev.is_none() {
        return false;
    }

    match (prev.unwrap(), next) {
        (
            Insert {
                pos: ppos,
                len: plen,
                ..
            },
            Insert { pos, eol, .. },
        ) => {
            let pend = ppos + plen;
            *eol || pend != *pos
        }
        (Remove { pos: ppos, .. }, Remove { pos, len }) => {
            let end = pos + len;
            *ppos != end
        }
        (Redo | Undo, _) => false,
        (_, Insert { eol, .. }) => *eol,
        _ => true,
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ChangeKind {
    Insert { pos: usize, len: usize, eol: bool },
    Remove { pos: usize, len: usize },
    Undo,
    Redo,
}

impl ChangeKind {
    pub fn insert<B: AsRef<[u8]>>(pos: usize, bytes: B) -> ChangeKind {
        let bytes = bytes.as_ref();
        let eol = EndOfLine::is_eol(bytes);
        let len = bytes.len();
        ChangeKind::Insert { pos, len, eol }
    }
}
