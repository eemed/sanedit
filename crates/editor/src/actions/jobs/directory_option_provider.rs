use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use sanedit_server::{BoxFuture, Kill};
use sanedit_utils::appendlist::Appendlist;
use tokio::{io, sync::oneshot};

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

async fn rayon_reader(dir: PathBuf, ctx: ReadDirContext) -> io::Result<()> {
    let (tx, rx) = oneshot::channel();
    get_option_provider_pool().spawn(|| {
        let _ = rayon_read(dir, ctx);
        let _ = tx.send(());
    });

    let _ = rx.await;
    Ok(())
}

fn rayon_read(dir: PathBuf, ctx: ReadDirContext) -> io::Result<()> {
    let mut rdir = std::fs::read_dir(&dir)?;
    while let Some(Ok(entry)) = rdir.next() {
        if ctx.kill.should_stop() {
            return Ok(());
        }

        let path = entry.path();
        let metadata = entry.metadata()?;
        if !metadata.is_dir() {
            continue;
        }

        if ctx.ignore.is_match(&path) {
            continue;
        }

        if ctx.recurse {
            let _ = rayon_read(dir.clone(), ctx.clone());
        }

        ctx.send(Choice::from_path(path, ctx.strip));
    }

    Ok(())
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
        recurse,
        n: n.clone(),
    };

    let _ = rayon_reader(dir, ctx).await;
    let n = n.load(Ordering::Acquire);
    done.store(n, Ordering::Release);
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
