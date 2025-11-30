use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use crossbeam::deque::{Injector, Steal, Worker};
use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};
use sanedit_server::{BoxFuture, Kill};
use sanedit_utils::appendlist::Appendlist;

use crate::{common::Choice, editor::ignore::Ignore};

use super::{get_option_provider_pool, OptionProvider};

#[derive(Clone)]
struct ReadDirContext {
    osend: Appendlist<Arc<Choice>>,
    strip: usize,
    kill: Kill,
    ignore: Ignore,
    recurse: bool,
    n: Arc<AtomicUsize>,
}
impl ReadDirContext {
    pub fn send(&self, opt: Arc<Choice>) {
        self.osend.append(opt);
        self.n.fetch_add(1, Ordering::Release);
    }
}

#[derive(Debug)]
pub(crate) struct DirectoryOptionProvider {
    path: PathBuf,
    ignore: Ignore,
    recurse: bool,
}

#[allow(dead_code)]
impl DirectoryOptionProvider {
    pub fn new(path: &Path, ignore: Ignore) -> DirectoryOptionProvider {
        DirectoryOptionProvider {
            path: path.to_owned(),
            ignore,
            recurse: true,
        }
    }

    pub fn new_non_recursive(path: &Path, ignore: Ignore) -> DirectoryOptionProvider {
        DirectoryOptionProvider {
            path: path.to_owned(),
            ignore,
            recurse: false,
        }
    }
}

fn read_directory(root: PathBuf, ctx: ReadDirContext) {
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
                        let Ok(metadata) = entry.metadata() else {
                            continue;
                        };
                        if !metadata.is_dir() || ctx.ignore.is_ignored(&path) {
                            continue;
                        }

                        if ctx.recurse {
                            injector.push(path.clone());
                        }

                        ctx.send(Choice::from_path(path, ctx.strip));
                    }
                }
            });
    });
}

async fn rayon_reader(dir: PathBuf, ctx: ReadDirContext) {
    tokio::task::spawn_blocking(move || read_directory(dir, ctx))
        .await
        .unwrap()
}

async fn read_directory_recursive(
    dir: PathBuf,
    osend: Appendlist<Arc<Choice>>,
    ignore: Ignore,
    kill: Kill,
    recurse: bool,
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
        strip,
        kill,
        ignore,
        recurse,
        n: n.clone(),
    };

    rayon_reader(dir, ctx).await;

    let count = n.load(Ordering::Acquire);
    done.store(count, Ordering::Release);
}

impl OptionProvider for DirectoryOptionProvider {
    fn provide(
        &self,
        sender: Appendlist<Arc<Choice>>,
        kill: Kill,
        done: Arc<AtomicUsize>,
    ) -> BoxFuture<'static, ()> {
        let dir = self.path.clone();
        Box::pin(read_directory_recursive(
            dir,
            sender,
            self.ignore.clone(),
            kill,
            self.recurse,
            done,
        ))
    }
}
