use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, OnceLock,
    },
};

use rayon::{
    iter::{IntoParallelIterator as _, ParallelIterator as _},
    ThreadPool,
};
use sanedit_utils::appendlist::Appendlist;

use sanedit_server::{BoxFuture, Kill};

use crate::{common::Choice, editor::ignore::Ignore};

use super::OptionProvider;
use crossbeam::deque::{Injector, Steal, Worker};

pub fn get_option_provider_pool() -> &'static ThreadPool {
    static POOL: OnceLock<ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        let parallelism = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let threads = parallelism.clamp(2, 8);

        log::debug!("Option provider pool: {threads} threads.");

        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|n| format!("option-provider-{n}"))
            .build()
            .unwrap()
    })
}

fn fast_parallel_read(root: PathBuf, ctx: ReadDirContext) {
    let injector = Injector::<PathBuf>::new();
    injector.push(root);

    get_option_provider_pool().install(|| {
        let threads = rayon::current_num_threads();
        (0..threads)
            .into_par_iter()
            .for_each_init(Worker::new_fifo, |local, _thread_idx| loop {
                if ctx.kill.should_stop() {
                    return;
                }

                let job = local
                    .pop()
                    .or_else(|| match injector.steal_batch_and_pop(local) {
                        Steal::Success(p) => Some(p),
                        _ => None,
                    });

                let Some(dir) = job else {
                    break;
                };

                if let Ok(mut rd) = std::fs::read_dir(&dir) {
                    while let Some(Ok(entry)) = rd.next() {
                        if ctx.kill.should_stop() {
                            return;
                        }

                        let path = entry.path();
                        if ctx.ignore.is_match(&path) {
                            continue;
                        }

                        let Ok(metadata) = entry.metadata() else {
                            continue;
                        };

                        if metadata.is_dir() {
                            injector.push(path);
                        } else if metadata.is_file() {
                            ctx.send(Choice::from_path(path, ctx.strip));
                        } else if metadata.is_symlink() {
                            if let Ok(cpath) = path.canonicalize() {
                                if cpath.is_file() {
                                    ctx.send(Choice::from_path(path, ctx.strip));
                                }
                            }
                        }
                    }
                }
            });
    });
}

async fn rayon_reader(dir: PathBuf, ctx: ReadDirContext) {
    tokio::task::spawn_blocking(move || fast_parallel_read(dir, ctx))
        .await
        .unwrap()
}

async fn read_directory_recursive(
    dir: PathBuf,
    osend: Appendlist<Arc<Choice>>,
    ignore: Ignore,
    kill: Kill,
    done: Arc<AtomicUsize>,
) {
    let strip = {
        let s = dir.as_os_str().to_string_lossy();
        let mut len = s.len();
        if !s.ends_with(std::path::MAIN_SEPARATOR) {
            len += 1;
        }
        len
    };

    let n = Arc::new(AtomicUsize::new(0));
    let ctx = ReadDirContext {
        osend,
        n: n.clone(),
        strip,
        kill,
        ignore,
    };

    rayon_reader(dir, ctx).await;

    let count = n.load(Ordering::Acquire);
    done.store(count, Ordering::Release);
}

impl OptionProvider for FileOptionProvider {
    fn provide(
        &self,
        sender: Appendlist<Arc<Choice>>,
        kill: Kill,
        done: Arc<AtomicUsize>,
    ) -> BoxFuture<'static, ()> {
        let dir = self.path.clone();
        let ignore = self.ignore.clone();
        Box::pin(read_directory_recursive(dir, sender, ignore, kill, done))
    }
}

#[derive(Clone)]
struct ReadDirContext {
    osend: Appendlist<Arc<Choice>>,
    n: Arc<AtomicUsize>,
    strip: usize,
    kill: Kill,
    ignore: Ignore,
}

impl ReadDirContext {
    pub fn send(&self, opt: Arc<Choice>) {
        self.osend.append(opt);
        self.n.fetch_add(1, Ordering::Release);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct FileOptionProvider {
    path: PathBuf,
    ignore: Ignore,
}

impl FileOptionProvider {
    pub fn new(path: &Path, ignore: Ignore) -> FileOptionProvider {
        FileOptionProvider {
            path: path.to_owned(),
            ignore,
        }
    }
}
