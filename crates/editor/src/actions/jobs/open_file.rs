use std::path::{Path, PathBuf};

use futures::future::BoxFuture;
use tokio::{
    fs, io,
    sync::{broadcast, mpsc::Sender},
};

use super::OptionProvider;

#[derive(Debug)]
pub(crate) struct FileOptionProvider {
    path: PathBuf,
}

impl FileOptionProvider {
    pub fn new(path: &Path) -> FileOptionProvider {
        FileOptionProvider {
            path: path.to_owned(),
        }
    }

    async fn read_directory_recursive(
        dir: PathBuf,
        osend: Sender<String>,
        kill: broadcast::Sender<()>,
    ) {
        fn spawn(dir: PathBuf, osend: Sender<String>, strip: usize, kill: broadcast::Sender<()>) {
            tokio::spawn(async move {
                let mut krecv = kill.subscribe();

                tokio::select! {
                     _ = read_recursive(dir, osend, strip, kill) => {}
                     _ = krecv.recv() => {}
                }
            });
        }

        async fn read_recursive(
            dir: PathBuf,
            osend: Sender<String>,
            strip: usize,
            kill: broadcast::Sender<()>,
        ) -> io::Result<()> {
            let mut rdir = fs::read_dir(&dir).await?;
            while let Ok(Some(entry)) = rdir.next_entry().await {
                let path = entry.path();
                let metadata = entry.metadata().await?;
                if metadata.is_dir() {
                    spawn(path, osend.clone(), strip, kill.clone());
                } else {
                    let path =
                        path.components()
                            .skip(strip)
                            .fold(PathBuf::new(), |mut acc, comp| {
                                acc.push(comp);
                                acc
                            });
                    let name: String = path.to_string_lossy().into();
                    let _ = osend.send(name).await;
                }
            }

            Ok(())
        }

        let strip = dir.components().count();
        let mut krecv = kill.subscribe();

        tokio::select! {
             _ = read_recursive(dir, osend, strip, kill) => {}
             _ = krecv.recv() => {}
        }
    }
}

impl OptionProvider for FileOptionProvider {
    fn provide(
        &self,
        sender: Sender<String>,
        kill: broadcast::Sender<()>,
    ) -> BoxFuture<'static, ()> {
        let dir = self.path.clone();
        Box::pin(Self::read_directory_recursive(dir, sender, kill))
    }
}
