use std::{
    any::Any,
    path::PathBuf,
    sync::{atomic::AtomicBool, mpsc, Arc},
};

use tokio::{
    fs, io,
    sync::mpsc::{channel, Receiver, Sender},
};

use crate::{
    editor::{jobs::Job, Editor},
    server::{ClientId, JobFutureFn, JobProgressSender},
};

pub(crate) const CHANNEL_SIZE: usize = 64;

async fn read_dir(out: Sender<String>, dir: PathBuf) -> bool {
    fn spawn(out: Sender<String>, dir: PathBuf) {
        tokio::spawn(read_dir(out, dir));
    }

    async fn read_recursive(out: Sender<String>, dir: PathBuf) -> io::Result<()> {
        let mut rdir = fs::read_dir(&dir).await?;
        while let Ok(Some(entry)) = rdir.next_entry().await {
            let path = entry.path();
            let metadata = entry.metadata().await?;
            if metadata.is_dir() {
                spawn(out.clone(), path)
            } else {
                let stripped = path.strip_prefix(&dir).unwrap();
                let name: String = stripped.to_string_lossy().into();
                let _ = out.send(name).await;
            }
        }

        Ok(())
    }

    read_recursive(out, dir).await.is_ok()
}

pub(crate) fn list_files(editor: &mut Editor, id: ClientId, term_in: Receiver<String>) -> Job {
    let dir = editor.working_dir().to_path_buf();
    let fun: JobFutureFn = { Box::new(move |send| Box::pin(list_files_task(dir, send, term_in))) };
    let on_output = Arc::new(|editor: &mut Editor, id: ClientId, out: Box<dyn Any>| {});
    let on_error = Arc::new(|editor: &mut Editor, id: ClientId, out: Box<dyn Any>| {});
    Job::new(id, fun).on_output(on_output).on_error(on_error)
}

async fn list_files_task(dir: PathBuf, out: JobProgressSender, term_in: Receiver<String>) -> bool {
    log::info!("list_files_task: {dir:?}");
    let (opt_out, opt_in) = channel(CHANNEL_SIZE);

    let (a, b) = tokio::join!(read_dir(opt_out, dir), handler(out, opt_in, term_in));

    log::info!("list_files_task done");
    a && b
}

/// Reads options and filter term from channels and send good results to
/// progress
async fn handler(
    out: JobProgressSender,
    mut opt_in: Receiver<String>,
    mut term_in: Receiver<String>,
) -> bool {
    log::info!("handler");
    const WORKER_COUNT: usize = 5;
    const BATCH_SIZE: usize = 512;

    rayon::scope(move |s| {
        // Spawn option receiver
        s.spawn(move |s1| {
            log::info!("Recv opts");
            let mut options: Vec<Arc<[String]>> = vec![];
            let mut block: Vec<String> = vec![];

            while let Some(opt) = opt_in.blocking_recv() {
                block.push(opt);

                if block.len() >= BATCH_SIZE {
                    let ablock: Arc<[String]> = block.into();
                    options.push(ablock.clone());
                    block = vec![];

                    // Spawn processing task
                    s1.spawn(move |_| {
                        log::info!("ABLOCK");
                    });
                }
            }

            if block.len() >= BATCH_SIZE {
                let ablock: Arc<[String]> = block.into();
                options.push(ablock.clone());

                // Spawn processing task
                s1.spawn(move |_| {
                    log::info!("ABLOCK");
                });
            }
        });
    });
    // Task to read options into array
    // when BATCH size options have arrived assign the matching work to a worker
    // thread. Worker thread reports back only the succesful matches and we send
    // them to out
    //
    // if term changes stop the workers and give them the new term
    // while let Some(term) = term_in.recv().await {
    //     log::info!("TERM: {term}");
    // }

    log::info!("handler done");
    true
}

async fn worker(recv: Receiver<ToWorker>, send: Sender<FromWorker>) {}
