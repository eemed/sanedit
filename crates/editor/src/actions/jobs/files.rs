use std::{
    any::Any,
    mem,
    path::PathBuf,
    sync::{mpsc, Arc},
};

use tokio::{
    fs, io,
    sync::mpsc::{channel, Receiver, Sender},
};

use crate::{
    actions::prompt,
    editor::{jobs::Job, Editor},
    server::{ClientId, JobFutureFn, JobProgress, JobProgressSender},
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
    let on_output = Arc::new(|editor: &mut Editor, id: ClientId, out: Box<dyn Any>| {
        if let Ok(mut output) = out.downcast::<MatcherResult>() {
            match output.as_mut() {
                MatcherResult::Reset => {
                    let (win, buf) = editor.win_buf_mut(id);
                    win.prompt.reset_selector();
                }
                MatcherResult::Options(opts) => {
                    prompt::provide_completions(editor, id, opts.to_owned())
                }
                _ => {}
            }
        }
    });
    let on_error = Arc::new(|editor: &mut Editor, id: ClientId, out: Box<dyn Any>| {});
    Job::new(id, fun).on_output(on_output).on_error(on_error)
}

async fn list_files_task(dir: PathBuf, out: JobProgressSender, term_in: Receiver<String>) -> bool {
    log::info!("list_files_task: {dir:?}");
    let (opt_out, opt_in) = channel(CHANNEL_SIZE);

    let (a, b) = tokio::join!(read_dir(opt_out, dir), matcher(out, opt_in, term_in));

    log::info!("list_files_task done");
    a && b
}

/// Reads options and filter term from channels and send good results to
/// progress
async fn matcher(
    mut out: JobProgressSender,
    mut opt_in: Receiver<String>,
    mut term_in: Receiver<String>,
) -> bool {
    log::info!("handler");
    const BATCH_SIZE: usize = 512;
    let worker_count = rayon::current_num_threads();
    let (sender, receiver) = mpsc::channel::<FromWorker>();
    let mut options: Vec<Arc<[String]>> = vec![];
    let mut block: Vec<String> = vec![];

    rayon::scope(move |s| {
        while let Some(opt) = opt_in.blocking_recv() {
            block.push(opt);

            if block.len() >= BATCH_SIZE {
                let ablock: Arc<[String]> = block.into();
                options.push(ablock.clone());
                block = vec![];

                // Spawn processing task
                // let job = BatchJob {
                //     term: pterm.clone(),
                //     batch: ablock,
                //     out: out.clone(),
                // };
                // s1.spawn(move |_| worker(job));
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
    //     log::info!("new term: {term}");
    //     out.send(JobProgress::Output(Box::new(MatcherResult::Reset)))
    //         .await;

    //     let pterm: Arc<String> = term.into();
    //     let oin = &mut opt_in;
    //     let out = &out;

    //     rayon::scope(move |s| {
    //         // Spawn option receiver
    //         s.spawn(move |s1| {
    //             let mut options: Vec<Arc<[String]>> = vec![];
    //             let mut block: Vec<String> = vec![];

    //             while let Some(opt) = oin.blocking_recv() {
    //                 block.push(opt);

    //                 if block.len() >= BATCH_SIZE {
    //                     let ablock: Arc<[String]> = block.into();
    //                     options.push(ablock.clone());
    //                     block = vec![];

    //                     // Spawn processing task
    //                     let job = BatchJob {
    //                         term: pterm.clone(),
    //                         batch: ablock,
    //                         out: out.clone(),
    //                     };
    //                     s1.spawn(move |_| worker(job));
    //                 }
    //             }

    //             if block.len() >= BATCH_SIZE {
    //                 let ablock: Arc<[String]> = block.into();
    //                 options.push(ablock.clone());

    //                 // Spawn processing task
    //                 let job = BatchJob {
    //                     term: pterm.clone(),
    //                     batch: ablock,
    //                     out: out.clone(),
    //                 };
    //                 s1.spawn(move |_| worker(job));
    //             }
    //         });
    //     });

    //     log::info!("calculated");
    // }

    log::info!("handler done");
    true
}

struct BatchJob {
    term: Arc<String>,
    batch: Arc<[String]>,
    out: JobProgressSender,
}

enum FromWorker {
    Done(Vec<String>),
}

enum ToWorker {
    Job(BatchJob),
}

pub(crate) enum MatcherResult {
    Reset,
    Options(Vec<String>),
}

fn worker(out: mpsc::Sender<FromWorker>, mut recv: mpsc::Receiver<ToWorker>) {
    fn matches(string: &str, input: &str, ignore_case: bool) -> Option<usize> {
        if ignore_case {
            string.to_ascii_lowercase().find(input)
        } else {
            string.find(input)
        }
    }

    while let Ok(msg) = recv.recv() {
        use ToWorker::*;
        match msg {
            Job(job) => {
                let mut ok = vec![];
                let term = job.term.as_ref();
                for opt in job.batch.iter() {
                    if opt.find(term).is_some() {
                        ok.push(opt.clone());
                    }
                }

                out.send(FromWorker::Done(ok));
            }
        }
    }
}

// fn worker(mut job: BatchJob) {
//     log::info!("New worker");
//     fn matches(string: &str, input: &str, ignore_case: bool) -> Option<usize> {
//         if ignore_case {
//             string.to_ascii_lowercase().find(input)
//         } else {
//             string.find(input)
//         }
//     }

//     let mut ok = vec![];
//     let term = job.term.as_ref();
//     for opt in job.batch.iter() {
//         if opt.find(term).is_some() {
//             ok.push(opt.clone());
//         }
//     }

//     if !ok.is_empty() {
//         let prog = JobProgress::Output(Box::new(MatcherResult::Options(ok)));
//         job.out.blocking_send(prog);
//     }
// }
