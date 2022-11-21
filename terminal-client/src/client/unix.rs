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

    pub fn run(self) {
        match (self.conn.try_clone(), self.conn.try_clone()) {
            (Ok(read), Ok(write)) => {
                super::run(read, write)
            }
            _ => {}
        }
    }
}
