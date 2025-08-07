use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use sanedit_server::{BoxFuture, Kill};
use tokio::{
    io,
    sync::{mpsc::Sender, oneshot},
};

use crate::{common::matcher::Choice, editor::ignore::Ignore};

use super::OptionProvider;

#[derive(Clone)]
struct ReadDirContext {
    osend: Sender<Arc<Choice>>,
    strip: usize,
    kill: Kill,
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

async fn rayon_reader(dir: PathBuf, ctx: ReadDirContext) -> io::Result<()> {
    let (tx, rx) = oneshot::channel();
    rayon::spawn(|| {
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
        let metadata = entry.metadata()?;
        if !metadata.is_dir() {
            continue;
        }

        if ctx.ignore.is_match(&path) {
            continue;
        }

        if ctx.recurse {
            let _ = rayon_read(scope, dir.clone(), ctx.clone());
        }

        let _ = ctx.osend.blocking_send(Choice::from_path(path, ctx.strip));
    }

    Ok(())
}

async fn read_directory_recursive(
    dir: PathBuf,
    osend: Sender<Arc<Choice>>,
    ignore: Ignore,
    kill: Kill,
    recurse: bool,
) {
    let strip = {
        let dir = dir.as_os_str().to_string_lossy();
        let mut len = dir.len();
        if !dir.ends_with(std::path::MAIN_SEPARATOR) {
            len += 1;
        }
        len
    };
    let ctx = ReadDirContext {
        osend,
        strip,
        kill,
        ignore,
        recurse,
    };

    let _ = rayon_reader(dir, ctx).await;
}

impl OptionProvider for DirectoryOptionProvider {
    fn provide(&self, sender: Sender<Arc<Choice>>, kill: Kill) -> BoxFuture<'static, ()> {
        let dir = self.path.clone();
        Box::pin(read_directory_recursive(
            dir,
            sender,
            self.ignore.clone(),
            kill,
            self.recurse,
        ))
    }
}
