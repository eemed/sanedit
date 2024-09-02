use sanedit_buffer::PieceTreeView;

/// Position that can be either offset into a buffer or LSP grid position
#[derive(Debug, Clone)]
pub enum Position {
    Offset(u64),
    LSP {
        position: lsp_types::Position,
        encoding: lsp_types::PositionEncodingKind,
    },
}

impl Position {
    pub fn to_offset(&self, buf: &PieceTreeView) -> u64 {
        match self {
            Position::Offset(off) => *off,
            Position::LSP { position, encoding } => position_to_offset(buf, *position, encoding),
        }
    }

    pub fn to_position(
        &self,
        buf: &PieceTreeView,
        encoding: &lsp_types::PositionEncodingKind,
    ) -> lsp_types::Position {
        match self {
            Position::Offset(off) => offset_to_position(buf, *off, encoding),
            Position::LSP {
                position,
                encoding: enc,
            } => {
                assert!(encoding == enc, "Position encoding mismatch");
                position.clone()
            }
        }
    }
}

pub(crate) fn offset_to_position(
    buf: &PieceTreeView,
    offset: u64,
    kind: &lsp_types::PositionEncodingKind,
) -> lsp_types::Position {
    let (row, line) = buf.line_at(offset);

    let mut chars = line.chars();
    let mut col = 0u32;

    while let Some((start, _, ch)) = chars.next() {
        if start > offset {
            break;
        }
        let len = if *kind == lsp_types::PositionEncodingKind::UTF8 {
            ch.len_utf8()
        } else if *kind == lsp_types::PositionEncodingKind::UTF16 {
            ch.len_utf16()
        } else if *kind == lsp_types::PositionEncodingKind::UTF32 {
            1
        } else {
            unreachable!("unsupported position encoding: {}", kind.as_str())
        };

        col += len as u32;
    }

    lsp_types::Position {
        line: row as u32,
        character: col,
    }
}

pub(crate) fn position_to_offset(
    buf: &PieceTreeView,
    position: lsp_types::Position,
    kind: &lsp_types::PositionEncodingKind,
) -> u64 {
    let lsp_types::Position { line, character } = position;
    let pos = buf.pos_at_line(line as u64);
    let slice = buf.slice(pos..);
    let line = slice
        .lines()
        .next()
        .expect("Position does not correspond to a line");

    let mut chars = line.chars();
    let mut col = 0u32;

    while let Some((start, _, ch)) = chars.next() {
        if col >= character {
            log::info!("char: {}", start);
            return start + line.start();
        }
        let len = if *kind == lsp_types::PositionEncodingKind::UTF8 {
            ch.len_utf8()
        } else if *kind == lsp_types::PositionEncodingKind::UTF16 {
            ch.len_utf16()
        } else if *kind == lsp_types::PositionEncodingKind::UTF32 {
            1
        } else {
            unreachable!("unsupported position encoding: {}", kind.as_str())
        };

        col += len as u32;
    }

    unreachable!("Position not found")
}
