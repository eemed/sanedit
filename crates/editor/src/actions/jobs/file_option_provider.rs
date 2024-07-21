use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use futures::future::BoxFuture;
use tokio::{
    fs, io,
    sync::{broadcast, mpsc::Sender},
};

use crate::common::matcher::MatchOption;

use super::OptionProvider;

#[derive(Clone)]
struct ReadDirContext {
    osend: Sender<MatchOption>,
    strip: usize,
    kill: broadcast::Sender<()>,
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
        let mut krecv = ctx.kill.subscribe();

        tokio::select! {
             _ = rec(dir, ctx) => {}
             _ = krecv.recv() => {}
        }
    });
}

async fn rec(dir: PathBuf, ctx: ReadDirContext) -> io::Result<()> {
    let mut rdir = fs::read_dir(&dir).await?;
    while let Ok(Some(entry)) = rdir.next_entry().await {
        let path = entry.path();
        let metadata = entry.metadata().await?;
        if metadata.is_dir() {
            spawn(path, ctx.clone());
        } else {
            let path = path
                .components()
                .skip(ctx.strip)
                .fold(PathBuf::new(), |mut acc, comp| {
                    acc.push(comp);
                    acc
                });
            let name: String = path.to_string_lossy().into();
            let _ = ctx.osend.send(name.into()).await;
        }
    }

    Ok(())
}

async fn read_directory_recursive(
    dir: PathBuf,
    osend: Sender<MatchOption>,
    ignore: Arc<Vec<String>>,
    kill: broadcast::Sender<()>,
) {
    let strip = dir.components().count();
    let mut krecv = kill.subscribe();
    let ctx = ReadDirContext {
        osend,
        strip,
        kill,
        ignore,
    };

    tokio::select! {
         _ = rec(dir, ctx) => {}
         _ = krecv.recv() => {}
    }
}

impl OptionProvider for FileOptionProvider {
    fn provide(
        &self,
        sender: Sender<MatchOption>,
        kill: broadcast::Sender<()>,
    ) -> BoxFuture<'static, ()> {
        let dir = self.path.clone();
        Box::pin(read_directory_recursive(
            dir,
            sender,
            self.ignore.clone(),
            kill,
        ))
    }
}
