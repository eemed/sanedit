use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, OnceLock,
    },
};

use rayon::ThreadPool;
use sanedit_utils::appendlist::Appendlist;
use tokio::{io, sync::oneshot};

use sanedit_server::{BoxFuture, Kill};

use crate::{common::Choice, editor::ignore::Ignore};

use super::OptionProvider;

pub fn get_option_provider_pool() -> &'static ThreadPool {
    static POOL: OnceLock<ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        const MIN: usize = 2;
        const MAX: usize = 4;
        let n = std::thread::available_parallelism()
            .map(|n| n.get() / 2)
            .unwrap_or(MIN)
            .clamp(MIN, MAX);
        log::info!("Starting option provider pool with {n} threads.");
        rayon::ThreadPoolBuilder::new()
            .thread_name(|n| format!("Option provider {n}"))
            .num_threads(n)
            .build()
            .unwrap()
    })
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

async fn rayon_reader(dir: PathBuf, ctx: ReadDirContext) -> io::Result<()> {
    let (tx, rx) = oneshot::channel();
    get_option_provider_pool().spawn(|| {
        rayon::scope(|s| {
            let _ = rayon_read(s, dir, ctx);
        });
        let _ = tx.send(());
    });

    let _ = rx.await;
    Ok(())
}

fn rayon_read(scope: &rayon::Scope, dir: PathBuf, ctx: ReadDirContext) -> io::Result<()> {
    let mut rdir = std::fs::read_dir(&dir)?;
    while let Some(Ok(entry)) = rdir.next() {
        if ctx.kill.should_stop() {
            return Ok(());
        }

        let path = entry.path();
        if ctx.ignore.is_match(&path) {
            continue;
        }

        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            let ctx = ctx.clone();
            scope.spawn(|s| {
                let _ = rayon_read(s, path, ctx);
            });
        } else if metadata.is_file() {
            let _ = ctx.send(Choice::from_path(path, ctx.strip));
        } else if metadata.is_symlink() {
            if let Ok(cpath) = path.canonicalize() {
                if cpath.is_file() {
                    let _ = ctx.send(Choice::from_path(path, ctx.strip));
                }
            }
        }
    }

    Ok(())
}

async fn read_directory_recursive(
    dir: PathBuf,
    osend: Appendlist<Arc<Choice>>,
    ignore: Ignore,
    kill: Kill,
    done: Arc<AtomicUsize>,
) {
    let strip = {
        let dir = dir.as_os_str().to_string_lossy();
        let mut len = dir.len();
        if !dir.ends_with(std::path::MAIN_SEPARATOR) {
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
        n: n.clone(),
    };

    let _ = rayon_reader(dir, ctx).await;
    let n = n.load(Ordering::Acquire);
    done.store(n, Ordering::Release);
}

impl OptionProvider for FileOptionProvider {
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
            done,
        ))
    }
}
