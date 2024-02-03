use std::{any::Any, path::PathBuf};

use tokio::{
    fs, io,
    sync::mpsc::{channel, Receiver, Sender},
};

use crate::{
    actions::jobs::match_options,
    editor::{job_broker::KeepInTouch, windows::SelectorOption, Editor},
    server::{BoxedJob, ClientId, Job, JobContext, JobResult},
};

use super::{MatchedOptions, CHANNEL_SIZE};

enum OpenFileMessage {
    Init(Sender<String>),
    Progress(MatchedOptions),
}

#[derive(Clone)]
pub(crate) struct OpenFile {
    client_id: ClientId,
    path: PathBuf,
}

impl OpenFile {
    pub fn new(id: ClientId, path: PathBuf) -> OpenFile {
        OpenFile {
            client_id: id,
            path,
        }
    }

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

    async fn send_matched_options(mut ctx: JobContext, mut mrecv: Receiver<MatchedOptions>) {
        while let Some(opts) = mrecv.recv().await {
            ctx.send(OpenFileMessage::Progress(opts)).await;
        }
    }
}

impl Job for OpenFile {
    fn run(&self, ctx: &JobContext) -> JobResult {
        log::info!("openfile");
        let mut ctx = ctx.clone();
        let dir = self.path.clone();

        let fut = async move {
            let (tsend, trecv) = channel::<String>(CHANNEL_SIZE);
            let (osend, orecv) = channel::<String>(CHANNEL_SIZE);
            let (msend, mrecv) = channel::<MatchedOptions>(CHANNEL_SIZE);

            ctx.send(OpenFileMessage::Init(tsend)).await;

            tokio::join!(
                Self::read_directory_recursive(dir, osend),
                match_options(orecv, trecv, msend),
                Self::send_matched_options(ctx, mrecv),
            );

            Ok(())
        };

        Box::pin(fut)
    }

    fn box_clone(&self) -> BoxedJob {
        Box::new((*self).clone())
    }
}

impl KeepInTouch for OpenFile {
    fn on_message(&self, editor: &mut Editor, msg: Box<dyn Any>) {
        let draw = editor.draw_state(self.client_id);
        draw.no_redraw_window();

        if let Ok(msg) = msg.downcast::<OpenFileMessage>() {
            let (win, buf) = editor.win_buf_mut(self.client_id);
            use OpenFileMessage::*;
            match *msg {
                Init(sender) => {
                    win.prompt.set_on_input(move |editor, id, input| {
                        let _ = sender.blocking_send(input.to_string());
                    });
                    win.prompt.clear_options();
                }
                Progress(opts) => match opts {
                    MatchedOptions::ClearAll => win.prompt.clear_options(),
                    MatchedOptions::Options(opts) => {
                        let opts: Vec<SelectorOption> = opts
                            .into_iter()
                            .map(|mat| SelectorOption::from(mat))
                            .collect();
                        win.prompt.provide_options(opts.into());
                    }
                },
            }
        }
    }

    fn client_id(&self) -> ClientId {
        self.client_id
    }
}
