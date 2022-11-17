mod input;
mod terminal;
mod client;

use std::path::{Path, PathBuf};

use tokio::{io::{self, AsyncReadExt}, net::UnixStream};

pub(crate) async fn run_unix_domain_socket<P: AsRef<Path>>(path: P) {
    let res = unix_domain_socket_loop(path.as_ref().to_path_buf());
}

pub(crate) async fn unix_domain_socket_loop(path: PathBuf) -> Result<(), io::Error> {
    let conn = UnixStream::connect(path).await?;
    let (read, write) = conn.split();

    Ok(())
}

fn conn_read(read: impl AsyncReadExt) {
}

fn conn_write(write: impl AsyncReadExt) {
}
