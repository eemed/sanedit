use std::{fs, path::PathBuf};

use crate::{
    actions::jobs::{MatchedOptions, MatcherMessage},
    editor::{
        windows::{Focus, SelectorOption},
        Editor,
    },
    server::ClientId,
};

pub(crate) fn matcher_result_handler(editor: &mut Editor, id: ClientId, msg: MatcherMessage) {
    use MatcherMessage::*;

    let draw = editor.draw_state(id);
    draw.no_redraw_window();

    let (win, _buf) = editor.win_buf_mut(id);
    match msg {
        Init(sender) => {
            win.prompt.set_on_input(move |editor, id, input| {
                let _ = sender.blocking_send(input.to_string());
            });
            win.prompt.clear_options();
        }
        Progress(opts) => match opts {
            MatchedOptions::Options { matched, clear_old } => {
                if clear_old {
                    win.prompt.clear_options();
                }

                win.focus = Focus::Prompt;
                let opts: Vec<SelectorOption> =
                    matched.into_iter().map(SelectorOption::from).collect();
                let (win, _buf) = editor.win_buf_mut(id);
                win.prompt.provide_options(opts.into());
            }
            _ => {}
        },
    }
}

pub(crate) fn on_confirm(editor: &mut Editor, id: ClientId, input: &str) {
    let path = {
        let mut path = editor.working_dir().to_path_buf();
        path.push(input);
        path
    };

    if path.is_dir() {
        // TODO Prompt again
    } else if !path.is_file() {
        let dir = {
            let mut path = path.clone();
            path.pop();
            path
        };

        if let Err(e) = fs::create_dir_all(&dir) {
            let (win, _buf) = editor.win_buf_mut(id);
            win.warn_msg(&format!(
                "Failed to create directories to {}: {e}",
                path.to_string_lossy()
            ));
            return;
        }

        if let Err(e) = std::fs::File::create(&path) {
            let (win, _buf) = editor.win_buf_mut(id);
            win.warn_msg(&format!(
                "Failed to create file to {}: {e}",
                path.to_string_lossy()
            ));
            return;
        }
    }

    log::info!("Path: {path:?}");
    if let Err(e) = editor.open_file(id, &path) {
        let (win, _buf) = editor.win_buf_mut(id);
        win.warn_msg(&format!("Failed to open file {input}"))
    }
}
