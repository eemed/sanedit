use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::{
    io,
    sync::{mpsc::Sender, oneshot},
};

use sanedit_server::{BoxFuture, Kill};

use crate::{common::matcher::Choice, editor::ignore::Ignore};

use super::OptionProvider;

#[derive(Clone)]
struct ReadDirContext {
    osend: Sender<Arc<Choice>>,
    strip: usize,
    kill: Kill,
    ignore: Ignore,
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
            let _ = ctx.osend.blocking_send(Choice::from_path(path, ctx.strip));
        } else if metadata.is_symlink() {
            if let Ok(cpath) = path.canonicalize() {
                if cpath.is_file() {
                    let _ = ctx.osend.blocking_send(Choice::from_path(path, ctx.strip));
                }
            }
        }
    }

    Ok(())
}

async fn read_directory_recursive(
    dir: PathBuf,
    osend: Sender<Arc<Choice>>,
    ignore: Ignore,
    kill: Kill,
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
    };

    let _ = rayon_reader(dir, ctx).await;
}

impl OptionProvider for FileOptionProvider {
    fn provide(&self, sender: Sender<Arc<Choice>>, kill: Kill) -> BoxFuture<'static, ()> {
        let dir = self.path.clone();
        Box::pin(read_directory_recursive(
            dir,
            sender,
            self.ignore.clone(),
            kill,
        ))
    }
}
