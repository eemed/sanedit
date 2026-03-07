use sanedit_messages::redraw::{status::Status, Redraw};
use sanedit_server::FromEditorSharedMessage;

use super::{DrawContext, EditorContext, Hash};

pub(crate) fn draw(ctx: &mut DrawContext) -> Option<FromEditorSharedMessage> {
    let statusline = draw_status(ctx);
    let hash = Hash::new(&statusline);
    if ctx.state.last_statusline.as_ref() == Some(&hash) {
        return None;
    }

    ctx.state.last_statusline = Some(hash);
    let redraw: Redraw = statusline.into();
    Some(FromEditorSharedMessage::from(redraw))
}

fn draw_status(ctx: &mut DrawContext) -> Status {
    let EditorContext {
        win,
        buf,
        working_dir,
        ..
    } = ctx.editor;

    let buffer = {
        match buf.path() {
            Some(path) => {
                let path = path.strip_prefix(working_dir).unwrap_or(path);
                path.to_string_lossy().to_string()
            }
            None => buf.name().to_string(),
        }
    };

    let cursor = win.primary_cursor();
    let cpos = cursor.pos();
    let blen = buf.len();
    let pressed_keys = win.keys().iter().fold(String::new(), |mut acc, ev| {
        if !acc.is_empty() {
            acc.push(' ');
        }
        acc.push_str(&ev.to_string());
        acc
    });
    let cursor_percentage = ((cpos as f64 / blen.max(1) as f64) * 100.0).floor() as usize;
    let language = buf
        .language
        .as_ref()
        .map(|lang| lang.as_str().to_string())
        .unwrap_or("no language".to_string());

    Status {
        buffer,
        buffer_modified: buf.is_modified(),
        buffer_read_only: buf.read_only,
        mode: win.mode,
        focus: win.focus(),
        cursor_percentage,
        macro_recording: win.macro_record.is_recording(),
        pressed_keys,
        language,
        end_of_line: buf.config.eol.name().to_string(),
        indent_kind: buf.config.indent_kind.as_ref().to_string(),
        indent_amount: buf.config.indent_amount as usize,
    }
}
