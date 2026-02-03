use sanedit_messages::redraw::{statusline::Statusline, Redraw};
use sanedit_server::FromEditorSharedMessage;

use crate::editor::windows::Focus;

use super::{DrawContext, EditorContext, Hash};

pub(crate) fn draw(ctx: &mut DrawContext) -> Option<FromEditorSharedMessage> {
    let statusline = draw_statusline(ctx);
    let hash = Hash::new(&statusline);
    if ctx.state.last_statusline.as_ref() == Some(&hash) {
        return None;
    }

    ctx.state.last_statusline = Some(hash);
    let redraw: Redraw = statusline.into();
    Some(FromEditorSharedMessage::from(redraw))
}

fn draw_statusline(ctx: &mut DrawContext) -> Statusline {
    let EditorContext {
        win,
        buf,
        working_dir,
        ..
    } = ctx.editor;

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
        left.push_str("(read-only) ");
    }

    let cursor = win.primary_cursor();
    let cpos = cursor.pos();
    let blen = buf.len();

    match win.focus() {
        Focus::Filetree => {
            left = " File browser".to_string();
        }
        Focus::Locations => {
            left = " Locations".to_string();
        }
        Focus::Snapshots => {
            left = " Undotree".to_string();
        }
        _ => {}
    }

    let right = {
        let mut result = String::new();
        let keys = win.keys();
        if !keys.is_empty() {
            let keys: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
            result.push_str(&keys.join(" "));
            result.push_str(" | ")
        }

        if win.macro_record.is_recording() {
            result.push_str(" REC | ")
        }

        result.push_str(&format!(
            " {} | {}% ",
            win.mode.statusline(),
            ((cpos as f64 / blen.max(1) as f64) * 100.0).floor()
        ));

        result
    };

    Statusline { left, right }
}
