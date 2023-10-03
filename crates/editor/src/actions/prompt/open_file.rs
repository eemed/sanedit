use std::{any::Any, path::PathBuf, rc::Rc, sync::Arc};

use tokio::{
    fs, io,
    sync::mpsc::{channel, Receiver, Sender},
};

use crate::{
    actions::prompt,
    common::matcher::CandidateMessage,
    editor::{
        jobs::Talkative,
        // jobs::Job,
        windows::{Focus, Prompt},
        Editor,
    },
    server::{ClientId, Job},
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

    log::info!("Open file");
    let path = editor.working_dir().to_path_buf();
    let job = OpenFile { id, path };
    editor.jobs.request(job);
    log::info!("Open file done");
}

enum OpenFileMessage {
    ResetOptions,
}

#[derive(Clone)]
struct OpenFile {
    id: ClientId,
    path: PathBuf,
}

impl OpenFile {
    async fn read_directory_recursive(dir: PathBuf, osend: Sender<String>) {
        fn spawn(dir: PathBuf, osend: Sender<String>, strip: usize) {
            tokio::spawn(read_recursive(dir, osend, strip));
        }

        async fn read_recursive(
            dir: PathBuf,
            osend: Sender<String>,
            strip: usize,
        ) -> io::Result<()> {
            let mut rdir = fs::read_dir(&dir).await?;
            while let Ok(Some(entry)) = rdir.next_entry().await {
                let path = entry.path();
                let metadata = entry.metadata().await?;
                if metadata.is_dir() {
                    spawn(path, osend.clone(), strip)
                } else {
                    let path =
                        path.components()
                            .skip(strip)
                            .fold(PathBuf::new(), |mut acc, comp| {
                                acc.push(comp);
                                acc
                            });
                    let name: String = path.to_string_lossy().into();
                    let _ = osend.send(name).await;
                }
            }

            Ok(())
        }

        let strip = dir.components().count();
        let _ = read_recursive(dir, osend, strip).await;
    }
}

impl Job for OpenFile {
    fn run(&self, ctx: &crate::server::JobContext) -> crate::server::JobResult {
        log::info!("Running openfile job..");
        let ctx = ctx.clone();
        let dir = self.path.clone();

        let fut = async move {
            log::info!("Running openfile job async block..");
            let (tsend, trecv) = channel::<String>(CHANNEL_SIZE);
            let (osend, orecv) = channel::<String>(CHANNEL_SIZE);
            Self::read_directory_recursive(dir, osend).await;
            // TODO: send required channels through ctx.send
            // atleast tsend needs to be sent to get the input updates
            //
            // let (a, b) = tokio::join!(
            //     Self::read_directory_recursive(dir, opt_out),
            //     matcher(out, opt_in, term_in)
            // );
            Ok(())
        };

        Box::pin(fut)
    }

    fn box_clone(&self) -> crate::server::BoxedJob {
        Box::new((*self).clone())
    }
}

impl Talkative for OpenFile {
    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>) {
        let draw = editor.draw_state(self.id);
        draw.no_redraw_window();

        // if let Ok(output) = out.downcast::<MatcherResult>() {
        //     match *output {
        //         MatcherResult::Reset => {
        //             let (win, _buf) = editor.win_buf_mut(id);
        //             win.prompt.reset_selector();
        //         }
        //         MatcherResult::Matches(opts) => prompt::provide_completions(editor, id, opts),
        //     }
        // }
    }
}
