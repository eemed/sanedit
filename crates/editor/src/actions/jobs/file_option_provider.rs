use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

use rayon::{
    iter::{IntoParallelIterator as _, ParallelIterator as _},
    ThreadPool,
};
use sanedit_syntax::GitGlob;

use sanedit_server::BoxFuture;

use crate::{
    common::{
        git::{find_git_root, GitIgnore, GitIgnoreList, GIT_IGNORE_FILENAME},
        Choice,
    },
    editor::ignore::Ignore,
};

use super::OptionProvider;
use crossbeam::{
    channel::Sender,
    deque::{Injector, Steal, Worker},
};

pub fn get_option_provider_pool() -> &'static ThreadPool {
    static POOL: OnceLock<ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        let parallelism = std::thread::available_parallelism()
            .map(|n| (n.get() as f64 * 0.7) as usize)
            .unwrap_or(1);

        let threads = parallelism.clamp(1, 8);

        log::debug!("Option provider pool: {threads} threads.");

        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|n| format!("option-provider-{n}"))
            .build()
            .unwrap()
    })
}

fn read_directory(root: PathBuf, mut ctx: ReadDirContext) {
    // If at already ignored directory, list everything
    if ctx.ignore.is_ignored(&root) {
        ctx.git_ignore = false;
    }

    let injector = Injector::<(PathBuf, Arc<GitIgnoreList>)>::new();
    injector.push((root, Arc::new(ctx.ignore.clone())));

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

                let Some((dir, mut ignore)) = job else {
                    break;
                };

                if ctx.git_ignore {
                    let ignore_path = dir.join(GIT_IGNORE_FILENAME);
                    if ignore_path.exists() {
                        if let Ok(git_ignore) = GitIgnore::new(&ignore_path) {
                            let ig = Arc::make_mut(&mut ignore);
                            ig.push(Arc::new(git_ignore));
                        }
                    }
                }

                if let Ok(mut rd) = std::fs::read_dir(&dir) {
                    while let Some(Ok(entry)) = rd.next() {
                        let path = entry.path();
                        if ignore.is_ignored(&path) {
                            continue;
                        }

                        let Ok(metadata) = entry.metadata() else {
                            continue;
                        };

                        if metadata.is_dir() {
                            injector.push((path, ignore.clone()));
                        } else if metadata.is_file() {
                            if ctx.sender.send(Choice::from_path(path, ctx.strip)).is_err() {
                                break;
                            }
                        } else if metadata.is_symlink() {
                            if let Ok(cpath) = path.canonicalize() {
                                if cpath.is_file()
                                    && ctx.sender.send(Choice::from_path(path, ctx.strip)).is_err()
                                {
                                    break;
                                }
                            }
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
    git_ignore: bool,
) {
    log::info!("File option provider: read directory");
    let strip = {
        let s = dir.as_os_str().to_string_lossy();
        let mut len = s.len();
        if !s.ends_with(std::path::MAIN_SEPARATOR) {
            len += 1;
        }
        len
    };

    let ignore = get_ignore(&dir, ignore, git_ignore);
    let ctx = ReadDirContext {
        sender,
        strip,
        ignore,
        git_ignore,
    };

    let _ = tokio::task::spawn_blocking(move || read_directory(dir, ctx)).await;

    log::info!("File option provider: read directory done");
}

fn get_ignore(dir: &Path, ignore: Ignore, git_ignore: bool) -> GitIgnoreList {
    let mut ignores = vec![ignore.into()];
    if !git_ignore {
        return GitIgnoreList::new(ignores);
    }

    ignores.push(Arc::new(GitIgnore {
        root: dir.to_path_buf(),
        patterns: vec![GitGlob::new(".git/").unwrap()],
    }));

    if let Ok((_root, git_ignores)) = find_git_root(dir) {
        for ignore in git_ignores {
            ignores.push(Arc::new(ignore));
        }
    }

    GitIgnoreList::new(ignores)
}

impl OptionProvider for FileOptionProvider {
    fn provide(&self, sender: Sender<Arc<Choice>>) -> BoxFuture<'static, ()> {
        let dir = self.path.clone();
        let ignore = self.ignore.clone();
        Box::pin(read_directory_recursive(
            dir,
            sender,
            ignore,
            self.git_ignore,
        ))
    }
}

#[derive(Clone)]
struct ReadDirContext {
    sender: Sender<Arc<Choice>>,
    strip: usize,
    ignore: GitIgnoreList,
    git_ignore: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct FileOptionProvider {
    path: PathBuf,
    ignore: Ignore,
    git_ignore: bool,
}

impl FileOptionProvider {
    pub fn new(path: &Path, ignore: Ignore, git_ignore: bool) -> FileOptionProvider {
        FileOptionProvider {
            path: path.to_owned(),
            ignore,
            git_ignore,
        }
    }
}
