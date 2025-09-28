use std::sync::{Arc, OnceLock};

use sanedit_buffer::utf8::EndOfLine;
use sanedit_messages::redraw::{text_style, Cell, PopupMessageText, Theme, ThemeField};
use sanedit_syntax::Parser;

use crate::editor::syntax::{Span, Syntax};

fn discard_parser() -> &'static Parser {
    static PARSER: OnceLock<Arc<Parser>> = OnceLock::new();
    let parser = PARSER.get_or_init(|| {
        let text = include_str!("../../pegs/markdown-discard.peg");
        let parser = Parser::new(std::io::Cursor::new(text)).unwrap();
        Arc::new(parser)
    });
    parser.as_ref()
}

pub(crate) fn render_markdown_to_popup(
    text: String,
    syn: &Syntax,
    theme: &Theme,
) -> PopupMessageText {
    match render_markdown(&text, syn, theme) {
        Ok(md) => PopupMessageText::Formatted(md),
        Err(e) => {
            log::error!("Failed to format markdown: {e}");
            PopupMessageText::Plain(text)
        }
    }
}

fn render_markdown(text: &str, syn: &Syntax, theme: &Theme) -> anyhow::Result<Vec<Vec<Cell>>> {
    let text = text.replace("\t", "    ");
    let parser = syn.get_parser();
    let captures = parser.parse(text.as_str())?;
    let mut highlights = Syntax::to_spans(0, parser, captures.captures);

    let mut stack = captures.injections;
    while let Some((lang, captures)) = stack.pop() {
        stack.extend(captures.injections);
        let loader = parser.loader.as_ref().unwrap();
        let inj_parser = loader.get(&lang).unwrap();
        let inj_spans = Syntax::to_spans(0, &inj_parser, captures.captures);
        highlights.merge(inj_spans)
    }

    let discard_parser = discard_parser();
    let discard_caps = discard_parser.parse(text.as_str())?;
    let discards = Syntax::to_spans(0, discard_parser, discard_caps.captures);

    let mut lines = to_lines(text.as_str(), &highlights, theme);
    lines = discard(&lines, &discards);
    strip_unnecessary_lines(&mut lines);
    Ok(lines)
}

fn discard(lines: &[Vec<Cell>], spans: &[Span]) -> Vec<Vec<Cell>> {
    let mut result = vec![vec![]; lines.len()];
    let mut spos = 0;
    let mut sline = 0;
    let mut scol = 0;

    'span: for span in spans {
        let range = span.range();
        let mut pos = spos;
        let row_offset = sline;
        let mut col_offset = scol;
        let mut start_found = false;

        for i in 0..(lines.len() - row_offset) {
            let line = row_offset + i;
            let row = &lines[line];
            for j in 0..(row.len().saturating_sub(col_offset)) {
                let col = col_offset + j;
                let cell = &lines[line][col];

                if range.contains(&pos) {
                    let mut cell = cell.clone();
                    cell.style.text_style = match span.name() {
                        "bold" => {
                            Some(text_style::BOLD | cell.style.text_style.unwrap_or_default())
                        }
                        "italic" => {
                            Some(text_style::ITALIC | cell.style.text_style.unwrap_or_default())
                        }
                        _ => cell.style.text_style,
                    };
                    result[line].push(cell);
                }

                if !start_found && range.start <= pos + cell.text.len() as u64 {
                    sline = line;
                    scol = col;
                    spos = pos;
                    start_found = true;
                }

                pos += cell.text.len() as u64;

                if range.end <= pos {
                    continue 'span;
                }
            }

            col_offset = 0;
        }
    }

    result
}

fn strip_unnecessary_lines(lines: &mut Vec<Vec<Cell>>) {
    let mut prev_was_empty = true;
    let mut i = 0;
    while i < lines.len() {
        let row = &mut lines[i];
        let mut delete = false;
        if let Some(last) = row.last() {
            if EndOfLine::is_eol(&last.text) {
                delete = true;
            }
        }

        if delete {
            row.pop();
        }

        if row.is_empty() && prev_was_empty {
            lines.remove(i);
            continue;
        }

        prev_was_empty = row.is_empty();
        i += 1;
    }

    let mut i = lines.len().saturating_sub(1);
    while i < lines.len() {
        let row = &mut lines[i];
        if !row.is_empty() {
            break;
        }

        lines.remove(i);
        i = i.saturating_sub(1);
    }
}

fn to_lines(text: &str, highlights: &[Span], theme: &Theme) -> Vec<Vec<Cell>> {
    const HL_PREFIX: &str = "window.view.";
    let base = theme.get(ThemeField::PopupDefault);
    let mut lines = vec![];
    let mut row = vec![];
    for ch in text.chars() {
        row.push(Cell::new_char(ch, base));
        if EndOfLine::is_eol_char(ch) {
            lines.push(std::mem::take(&mut row));
        }
    }

    let mut spos = 0;
    let mut sline = 0;
    let mut scol = 0;

    'hls: for highlight in highlights {
        let style = {
            let key = format!("{}{}", HL_PREFIX, highlight.name());
            let mut style = theme.get(&key);
            style.bg = base.bg.or(style.bg);
            style
        };

        let mut start_found = false;
        let mut pos = spos;
        let row_offset = sline;
        let mut col_offset = scol;

        for i in 0..(lines.len() - row_offset) {
            let line = row_offset + i;
            let row = &mut lines[line];
            for j in 0..(row.len() - col_offset) {
                let col = col_offset + j;
                let cell = &mut lines[line][col];
                if highlight.range().contains(&pos) {
                    cell.style = style;
                }

                if !start_found && highlight.start() <= pos + cell.text.len() as u64 {
                    sline = line;
                    scol = col;
                    spos = pos;
                    start_found = true;
                }

                pos += cell.text.len() as u64;

                if pos >= highlight.end() {
                    continue 'hls;
                }
            }

            col_offset = 0;
        }
    }

    lines
}
