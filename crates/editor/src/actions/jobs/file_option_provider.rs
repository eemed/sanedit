use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use sanedit_core::Choice;
use tokio::{fs, io, sync::mpsc::Sender};

use sanedit_server::{BoxFuture, Kill};

use crate::common::matcher::PathChoice;

use super::OptionProvider;

#[derive(Clone)]
struct ReadDirContext {
    osend: Sender<Arc<dyn Choice>>,
    strip: usize,
    kill: Kill,
    ignore: Arc<Vec<String>>,
}

#[derive(Debug, Clone)]
pub(crate) struct FileOptionProvider {
    path: PathBuf,
    ignore: Arc<Vec<String>>,
}

impl FileOptionProvider {
    pub fn new(path: &Path, ignore: &[String]) -> FileOptionProvider {
        FileOptionProvider {
            path: path.to_owned(),
            ignore: Arc::new(ignore.into()),
        }
    }
}

fn spawn(dir: PathBuf, ctx: ReadDirContext) {
    if let Some(fname) = dir.file_name().map(|fname| fname.to_string_lossy()) {
        for ig in ctx.ignore.iter() {
            if ig.as_str() == fname {
                return;
            }
        }
    }

    tokio::spawn(async move {
        let _ = rec(dir, ctx).await;
    });
}

async fn rec(dir: PathBuf, ctx: ReadDirContext) -> io::Result<()> {
    let mut rdir = fs::read_dir(&dir).await?;
    while let Ok(Some(entry)) = rdir.next_entry().await {
        if ctx.kill.should_stop() {
            return Ok(());
        }

        let path = entry.path();
        let metadata = entry.metadata().await?;
        if metadata.is_dir() {
            spawn(path, ctx.clone());
        } else {
            let opt = PathChoice::new(path, ctx.strip);
            let _ = ctx.osend.send(Arc::new(opt)).await;
        }
    }

    Ok(())
}

async fn read_directory_recursive(
    dir: PathBuf,
    osend: Sender<Arc<dyn Choice>>,
    ignore: Arc<Vec<String>>,
    kill: Kill,
) {
    let strip = dir.as_os_str().len() + 1;
    let ctx = ReadDirContext {
        osend,
        strip,
        kill,
        ignore,
    };

    let _ = rec(dir, ctx).await;
}

impl OptionProvider for FileOptionProvider {
    fn provide(&self, sender: Sender<Arc<dyn Choice>>, kill: Kill) -> BoxFuture<'static, ()> {
        let dir = self.path.clone();
        Box::pin(read_directory_recursive(
            dir,
            sender,
            self.ignore.clone(),
            kill,
        ))
    }
}
