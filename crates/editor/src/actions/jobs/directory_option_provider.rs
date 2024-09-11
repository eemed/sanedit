use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use futures::future::BoxFuture;
use sanedit_core::{Kill, MatchOption};
use tokio::{fs, io, sync::mpsc::Sender};

use super::OptionProvider;

#[derive(Clone)]
struct ReadDirContext {
    osend: Sender<MatchOption>,
    strip: usize,
    kill: Kill,
    ignore: Arc<Vec<String>>,
}

#[derive(Debug)]
pub(crate) struct DirectoryOptionProvider {
    path: PathBuf,
    ignore: Arc<Vec<String>>,
}

impl DirectoryOptionProvider {
    pub fn new(path: &Path, ignore: &[String]) -> DirectoryOptionProvider {
        DirectoryOptionProvider {
            path: path.to_owned(),
            ignore: Arc::new(ignore.into()),
        }
    }
}

async fn list_dirs(dir: PathBuf, ctx: ReadDirContext) -> io::Result<()> {
    let mut rdir = fs::read_dir(&dir).await?;
    while let Ok(Some(entry)) = rdir.next_entry().await {
        if ctx.kill.should_stop() {
            return Ok(());
        }

        let path = entry.path();
        let metadata = entry.metadata().await?;
        if metadata.is_dir() {
            if let Some(fname) = dir.file_name().map(|fname| fname.to_string_lossy()) {
                for ig in ctx.ignore.iter() {
                    if ig.as_str() == fname {
                        continue;
                    }
                }
            }

            let path = path
                .components()
                .skip(ctx.strip)
                .fold(PathBuf::new(), |mut acc, comp| {
                    acc.push(comp);
                    acc
                });

            let _ = ctx.osend.send(path.into()).await;
        }
    }

    Ok(())
}

async fn read_directory_recursive(
    dir: PathBuf,
    osend: Sender<MatchOption>,
    ignore: Arc<Vec<String>>,
    kill: Kill,
) {
    let strip = dir.components().count();
    let ctx = ReadDirContext {
        osend,
        strip,
        kill,
        ignore,
    };

    list_dirs(dir, ctx).await;
}

impl OptionProvider for DirectoryOptionProvider {
    fn provide(&self, sender: Sender<MatchOption>, kill: Kill) -> BoxFuture<'static, ()> {
        let dir = self.path.clone();
        Box::pin(read_directory_recursive(
            dir,
            sender,
            self.ignore.clone(),
            kill,
        ))
    }
}
