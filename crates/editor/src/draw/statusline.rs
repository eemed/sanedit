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

    if win.focus() == Focus::Filetree {
        let left = " File browser".to_string();
        let right = String::new();
        return Statusline { left, right };
    }

    let name = {
        match buf.path() {
            Some(path) => {
                let path = path.strip_prefix(working_dir).unwrap_or(path);
                path.to_string_lossy()
            }
            None => buf.name(),
        }
    };
    let mut left = format!(" {} ", name);
    if buf.is_modified() {
        left.push_str("* ");
    }
    if buf.read_only {
        left.push_str("RO ");
    }

    let cursor = win.primary_cursor();
    let cpos = cursor.pos();
    let blen = buf.len();

    let right = {
        let mut result = String::new();
        let keys = win.keys();
        if !keys.is_empty() {
            let keys: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
            result.push_str(&keys.join(" "));
            result.push_str(" | ")
        }


        result.push_str(&format!(" {} | ", win.mode.statusline()));
        result.push_str(&format!(
            "{}% ",
            ((cpos as f64 / blen.max(1) as f64) * 100.0).floor(),
            // to_human_readable(blen as f64)
        ));

        result
    };

    Statusline { left, right }
}
