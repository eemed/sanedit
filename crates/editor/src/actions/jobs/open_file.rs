use std::{any::Any, path::PathBuf};

use tokio::{
    fs, io,
    sync::{
        broadcast,
        mpsc::{channel, Receiver, Sender},
    },
};

use crate::{
    actions::jobs::match_options,
    editor::{job_broker::KeepInTouch, windows::SelectorOption, Editor},
    job_runner::{Job, JobContext, JobId, JobResponseSender, JobResult},
    server::ClientId,
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

    async fn read_directory_recursive(
        dir: PathBuf,
        osend: Sender<String>,
        kill: broadcast::Sender<()>,
    ) {
        fn spawn(dir: PathBuf, osend: Sender<String>, strip: usize, kill: broadcast::Sender<()>) {
            tokio::spawn(async move {
                let mut krecv = kill.subscribe();

                tokio::select! {
                     _ = read_recursive(dir, osend, strip, kill) => {}
                     _ = krecv.recv() => {}
                }
            });
        }

        async fn read_recursive(
            dir: PathBuf,
            osend: Sender<String>,
            strip: usize,
            kill: broadcast::Sender<()>,
        ) -> io::Result<()> {
            let mut rdir = fs::read_dir(&dir).await?;
            while let Ok(Some(entry)) = rdir.next_entry().await {
                let path = entry.path();
                let metadata = entry.metadata().await?;
                if metadata.is_dir() {
                    spawn(path, osend.clone(), strip, kill.clone());
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
        let mut krecv = kill.subscribe();

        tokio::select! {
             _ = read_recursive(dir, osend, strip, kill) => {}
             _ = krecv.recv() => {}
        }
    }

    async fn send_matched_options(
        id: JobId,
        mut sender: JobResponseSender,
        mut mrecv: Receiver<MatchedOptions>,
    ) {
        while let Some(opts) = mrecv.recv().await {
            sender.send(id, OpenFileMessage::Progress(opts));
        }
    }
}

impl Job for OpenFile {
    fn run(&self, ctx: JobContext) -> JobResult {
        let dir = self.path.clone();

        let fut = async move {
            let JobContext {
                id,
                kill,
                mut sender,
            } = ctx;

            // Kill channel
            let (ksend, _krecv) = broadcast::channel(1);
            // Term channel
            let (tsend, trecv) = channel::<String>(CHANNEL_SIZE);
            // Options channel
            let (osend, orecv) = channel::<String>(CHANNEL_SIZE);
            // Messages channel
            let (msend, mrecv) = channel::<MatchedOptions>(CHANNEL_SIZE);

            sender.send(id, OpenFileMessage::Init(tsend));

            // Broadcast the kill signal to the directory reader tasks to kill
            // them all
            let ksend2 = ksend.clone();
            tokio::spawn(async move {
                let _ = kill.await;
                let _ = ksend2.send(());
            });

            tokio::join!(
                Self::read_directory_recursive(dir, osend, ksend),
                match_options(orecv, trecv, msend),
                Self::send_matched_options(id, sender, mrecv),
            );

            Ok(())
        };

        Box::pin(fut)
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
                        let opts: Vec<SelectorOption> =
                            opts.into_iter().map(SelectorOption::from).collect();
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
