use std::{
    io,
    os::unix::net::UnixStream,
    path::Path,
    sync::mpsc::{self, Sender},
    thread,
};

use crate::input;

pub struct UnixDomainSocketClient {
    conn: UnixStream,
}

impl UnixDomainSocketClient {
    pub fn connect<P: AsRef<Path>>(path: P) -> io::Result<UnixDomainSocketClient> {
        let conn = UnixStream::connect(path)?;
        Ok(UnixDomainSocketClient { conn })
    }
}
