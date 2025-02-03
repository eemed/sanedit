use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use sanedit_server::{BoxFuture, Kill};
use tokio::{
    io,
    sync::{mpsc::Sender, oneshot},
};

use crate::common::matcher::Choice;

use super::OptionProvider;

#[derive(Clone)]
struct ReadDirContext {
    osend: Sender<Arc<Choice>>,
    strip: usize,
    kill: Kill,
    ignore: Arc<Vec<String>>,
}

#[derive(Debug)]
pub(crate) struct DirectoryOptionProvider {
    path: PathBuf,
    ignore: Arc<Vec<String>>,
}

#[allow(dead_code)]
impl DirectoryOptionProvider {
    pub fn new(path: &Path, ignore: &[String]) -> DirectoryOptionProvider {
        DirectoryOptionProvider {
            path: path.to_owned(),
            ignore: Arc::new(ignore.into()),
        }
    }
}

async fn rayon_reader(dir: PathBuf, ctx: ReadDirContext) -> io::Result<()> {
    let (tx, rx) = oneshot::channel();
    rayon::spawn(|| {
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

            let _ = ctx.osend.blocking_send(Choice::from_path(path, ctx.strip));
        }
    }

    Ok(())
}

async fn read_directory_recursive(
    dir: PathBuf,
    osend: Sender<Arc<Choice>>,
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
        ))
    }
}
