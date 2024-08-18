use sanedit_buffer::ReadOnlyPieceTree;

pub(crate) fn offset_to_position(
    buf: &ReadOnlyPieceTree,
    offset: usize,
    kind: lsp_types::PositionEncodingKind,
) -> lsp_types::Position {
    let mut row = 0;
    let mut line = buf.slice(..);
    let mut lines = buf.lines();

    while let Some(next_line) = lines.next() {
        if next_line.range().contains(&offset) {
            line = next_line;
            break;
        }
        row += 1;
    }

    let mut chars = line.chars();
    let mut col = 0u32;

    while let Some((start, _, ch)) = chars.next() {
        if start > offset {
            break;
        }
        let len = if kind == lsp_types::PositionEncodingKind::UTF8 {
            ch.len_utf8()
        } else if kind == lsp_types::PositionEncodingKind::UTF16 {
            ch.len_utf16()
        } else if kind == lsp_types::PositionEncodingKind::UTF32 {
            1
        } else {
            unreachable!("unsupported position encoding: {}", kind.as_str())
        };

        col += len as u32;
    }

    lsp_types::Position {
        line: row,
        character: col,
    }
}

pub(crate) fn position_to_offset(
    buf: &ReadOnlyPieceTree,
    position: lsp_types::Position,
    kind: lsp_types::PositionEncodingKind,
) -> usize {
    todo!()
}
