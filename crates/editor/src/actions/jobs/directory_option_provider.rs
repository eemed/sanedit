use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crossbeam::{
    channel::Sender,
    deque::{Injector, Steal, Worker},
};
use rayon::iter::{IntoParallelIterator as _, ParallelIterator as _};
use sanedit_server::BoxFuture;

use crate::{common::Choice, editor::ignore::Ignore};

use super::{get_option_provider_pool, OptionProvider};

#[derive(Clone)]
struct ReadDirContext {
    sender: Sender<Arc<Choice>>,
    strip: usize,
    ignore: Ignore,
    recurse: bool,
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

                        if ctx.sender.send(Choice::from_path(path, ctx.strip)).is_err() {
                            break;
                        }
                    }
                }
            });
    });
}

async fn read_directory_recursive(
    dir: PathBuf,
    sender: Sender<Arc<Choice>>,
    ignore: Ignore,
    recurse: bool,
) {
    let strip = {
        let s = dir.as_os_str().to_string_lossy();
        let mut len = s.len();
        if !s.ends_with(std::path::MAIN_SEPARATOR) {
            len += 1;
        }
        len
    };

    let ctx = ReadDirContext {
        sender,
        strip,
        ignore,
        recurse,
    };

    let _ = tokio::task::spawn_blocking(move || read_directory(dir, ctx)).await;
}

impl OptionProvider for DirectoryOptionProvider {
    fn provide(&self, sender: Sender<Arc<Choice>>) -> BoxFuture<'static, ()> {
        let dir = self.path.clone();
        Box::pin(read_directory_recursive(
            dir,
            sender,
            self.ignore.clone(),
            self.recurse,
        ))
    }
}
