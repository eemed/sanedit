use std::{
    any::Any,
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
};

use crossbeam::deque::{Injector, Worker};
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
    async fn read_recursive(out: Sender<String>, base: PathBuf) -> io::Result<()> {
        let mut stack: Vec<PathBuf> = Vec::new();
        stack.push(base.clone());

        while let Some(dir) = stack.pop() {
            let mut read_dir = fs::read_dir(&dir).await?;
            while let Ok(Some(entry)) = read_dir.next_entry().await {
                let path = entry.path();
                let metadata = entry.metadata().await?;
                if metadata.is_dir() {
                    stack.push(path);
                } else {
                    let stripped = path.strip_prefix(&base).unwrap();
                    let name: String = stripped.to_string_lossy().into();
                    out.send(name);
                }
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
    log::info!("list_files_task");
    let (opt_out, opt_in) = channel(CHANNEL_SIZE);

    // handle term changes while options are coming in
    handler(out, opt_in, term_in).await;

    log::info!("list_files_task done");
    true
}

type WorkFn = Arc<dyn Fn(Sender<FromWorker>, AtomicBool)>;

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

    let injector = Injector::<Arc<[String]>>::new();
    let worker1 = Worker::<Arc<[String]>>::new_fifo();
    let worker2 = Worker::<Arc<[String]>>::new_fifo();

    let stealers = [worker1.stealer(), worker2.stealer()];

    tokio::spawn(async move { worker1.pop() });
    tokio::spawn(async move { worker2.pop() });

    tokio::spawn(async move {
        let mut options: Vec<Arc<[String]>> = vec![];
        let mut block: Vec<String> = vec![];

        while let Some(opt) = opt_in.recv().await {
            block.push(opt);

            if block.len() >= BATCH_SIZE {
                let ablock: Arc<[String]> = block.into();
                injector.push(ablock.clone());
                options.push(ablock);
                block = vec![];
            }
        }
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

enum ToWorker {
    Work,
    Stop,
}

enum FromWorker {
    MoreWork,
}

async fn worker(recv: Receiver<ToWorker>, send: Sender<FromWorker>) {}
