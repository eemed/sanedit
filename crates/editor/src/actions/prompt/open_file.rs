use std::{any::Any, path::PathBuf, rc::Rc, sync::Arc};

use tokio::{
    fs, io,
    sync::mpsc::{channel, Receiver, Sender},
};

use crate::{
    actions::prompt,
    common::matcher::CandidateMessage,
    editor::{
        // jobs::Job,
        windows::{Focus, Prompt},
        Editor,
    },
    server::ClientId,
};

const CHANNEL_SIZE: usize = 64;

#[action("Open a file")]
fn open_file(editor: &mut Editor, id: ClientId) {
    // let (tx, rx) = channel(CHANNEL_SIZE);
    // let job = list_files(editor, id, rx);
    // let (win, _buf) = editor.win_buf_mut(id);

    // win.prompt = Prompt::new("Open a file");
    // win.prompt.on_input = Some(Rc::new(move |editor, id, input| {
    //     let _ = tx.blocking_send(input.into());
    // }));
    // win.prompt.on_confirm = Some(Rc::new(move |editor, id, input| {
    //     let (win, _buf) = editor.win_buf_mut(id);
    //     win.prompt.on_input = None;
    //     let path = PathBuf::from(input);

    //     if let Err(e) = editor.open_file(id, &path) {
    //         let (win, _buf) = editor.win_buf_mut(id);
    //         win.warn_msg(&format!("Failed to open file {input}"))
    //     }
    // }));
    // win.prompt.on_abort = Some(Rc::new(move |editor, id, input| {
    //     let (win, _buf) = editor.win_buf_mut(id);
    //     win.prompt.on_input = None;
    // }));
    // win.focus = Focus::Prompt;

    // editor.jobs.request(job);
}

// fn list_files(editor: &mut Editor, id: ClientId, term_in: Receiver<String>) -> Job {
//     let dir = editor.working_dir().to_path_buf();
//     let fun: JobFutureFn = { Box::new(move |send| Box::pin(list_files_task(dir, send, term_in))) };
//     let mut job = Job::new(id, fun);
//     job.on_output = Some(Arc::new(
//         |editor: &mut Editor, id: ClientId, out: Box<dyn Any>| {
//             let draw = editor.draw_state(id);
//             draw.no_redraw_window();

//             if let Ok(output) = out.downcast::<MatcherResult>() {
//                 match *output {
//                     MatcherResult::Reset => {
//                         let (win, _buf) = editor.win_buf_mut(id);
//                         win.prompt.reset_selector();
//                     }
//                     MatcherResult::Matches(opts) => prompt::provide_completions(editor, id, opts),
//                 }
//             }
//         },
//     ));
//     job.on_error = Some(Arc::new(
//         |editor: &mut Editor, id: ClientId, out: Box<dyn Any>| {},
//     ));
//     job
// }

// async fn list_files_task(dir: PathBuf, out: JobProgressSender, term_in: Receiver<String>) -> bool {
//     let (opt_out, opt_in) = channel(CHANNEL_SIZE);
//     let (a, b) = tokio::join!(read_dir(opt_out, dir), matcher(out, opt_in, term_in));
//     a && b
// }

// async fn read_dir(out: Sender<CandidateMessage>, dir: PathBuf) -> bool {
//     fn spawn(out: Sender<CandidateMessage>, dir: PathBuf, strip: usize) {
//         tokio::spawn(read_recursive(out, dir, strip));
//     }

//     async fn read_recursive(
//         out: Sender<CandidateMessage>,
//         dir: PathBuf,
//         strip: usize,
//     ) -> io::Result<()> {
//         let mut rdir = fs::read_dir(&dir).await?;
//         while let Ok(Some(entry)) = rdir.next_entry().await {
//             let path = entry.path();
//             let metadata = entry.metadata().await?;
//             if metadata.is_dir() {
//                 spawn(out.clone(), path, strip)
//             } else {
//                 let path = path
//                     .components()
//                     .skip(strip)
//                     .fold(PathBuf::new(), |mut acc, comp| {
//                         acc.push(comp);
//                         acc
//                     });
//                 let name: String = path.to_string_lossy().into();
//                 let _ = out.send(CandidateMessage::One(name)).await;
//             }
//         }

//         Ok(())
//     }

//     let strip = dir.components().count();
//     read_recursive(out, dir, strip).await.is_ok()
// }
