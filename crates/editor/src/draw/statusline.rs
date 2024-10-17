use sanedit_core::Filetype;
use sanedit_messages::redraw::statusline::Statusline;

use crate::editor::windows::Focus;

use super::{DrawContext, EditorContext};

pub(crate) fn draw(ctx: &mut DrawContext) -> Statusline {
    let EditorContext {
        win,
        buf,
        working_dir,
        ..
    } = ctx.editor;

    if win.focus == Focus::Filetree {
        let left = " File browser".to_string();
        let right = String::new();
        return Statusline { left, right };
    }

    let name = buf.name();
    let wd = format!(
        "{}{}",
        working_dir.to_string_lossy().as_ref(),
        std::path::MAIN_SEPARATOR_STR
    );
    let mut left = match name.strip_prefix(&wd) {
        Some(bname) => format!(" {} ", bname),
        None => format!(" {} ", name),
    };
    if buf.is_modified() {
        left.push_str("* ");
    }
    if buf.read_only {
        left.push_str("RO ");
    }

    let cursor = win.primary_cursor();
    let cpos = cursor.pos();
    let blen = buf.len();
    let filetype = buf.filetype.as_ref();
    let ft = filetype.map(Filetype::as_str).unwrap_or("no filetype");
    let lsp = filetype.and_then(|ft| {
        ctx.editor
            .language_servers
            .get(ft)
            .map(|lsp| lsp.server_name().to_string())
    });

    let right = {
        let mut result = String::new();
        let keys = win.keys();
        let persist = win.key_persist;
        if !keys.is_empty() {
            let keys: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
            if persist != 0 {
                result.push('(');
                result.push_str(&keys[..persist].join(" "));
                result.push(')');
                result.push(' ');
            }

            result.push_str(&keys[persist..].join(" "));
            result.push_str(" | ")
        }

        if let Some(lsp) = lsp {
            result.push_str(&format!(" {lsp} | "));
        }

        result.push_str(&format!(
            "{ft} | {}% {cpos}/{blen} ",
            ((cpos as f64 / blen.max(1) as f64) * 100.0).floor()
        ));

        result
    };

    Statusline { left, right }
}
