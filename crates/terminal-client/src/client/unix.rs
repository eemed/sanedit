use std::{
    io,
    os::unix::net::UnixStream,
    path::Path,
    sync::mpsc::{self, Sender},
    thread,
};

use crate::input;

#[derive(Debug)]
struct UnixDomainSocket(UnixStream);

impl Clone for UnixDomainSocket {
    fn clone(&self) -> Self {
        let conn = self
            .0
            .try_clone()
            .expect("Failed to clone unix domain socket");
        UnixDomainSocket(conn)
    }
}

impl io::Read for UnixDomainSocket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl io::Write for UnixDomainSocket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

pub struct UnixDomainSocketClient {
    socket: UnixDomainSocket,
}

impl UnixDomainSocketClient {
    pub fn connect<P: AsRef<Path>>(path: P) -> io::Result<UnixDomainSocketClient> {
        let conn = UnixStream::connect(path)?;
        let socket = UnixDomainSocket(conn);
        Ok(UnixDomainSocketClient { socket })
    }

    pub fn run(self) {
        let read = self.socket.clone();
        let write = self.socket;
        crate::run(read, write);
    }
}
