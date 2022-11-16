use std::path::PathBuf;

use tokio::{
    io::{self, AsyncWriteExt},
    net::{unix::SocketAddr, TcpStream, UnixStream},
};

#[derive(Debug)]
pub enum Address {
    UnixDomainSocket(PathBuf),
    Tcp(SocketAddr),
}

#[derive(Debug)]
pub enum Connection {
    UnixDomainSocket(UnixStream),
    Tcp(TcpStream),
}

pub enum ReadHalf<'a> {
    UnixDomainSocket(tokio::net::unix::ReadHalf<'a>),
    Tcp(tokio::net::tcp::ReadHalf<'a>),
}

pub enum WriteHalf<'a> {
    UnixDomainSocket(tokio::net::unix::WriteHalf<'a>),
    Tcp(tokio::net::tcp::WriteHalf<'a>),
}

impl Connection {
    pub fn split<'a>(&'a mut self) -> (ReadHalf<'a>, WriteHalf<'a>) {
        match self {
            Connection::UnixDomainSocket(chan) => {
                let (read, write) = chan.split();
                (
                    ReadHalf::UnixDomainSocket(read),
                    WriteHalf::UnixDomainSocket(write),
                )
            }
            Connection::Tcp(tcp) => {
                let (read, write) = tcp.split();
                (ReadHalf::Tcp(read), WriteHalf::Tcp(write))
            }
        }
    }

    pub async fn shutdown(&mut self) -> Result<(), io::Error> {
        match self {
            Connection::UnixDomainSocket(chan) => chan.shutdown().await,
            Connection::Tcp(tcp) => tcp.shutdown().await,
        }
    }
}

#[derive(Debug)]
pub struct Client {
    conn: Connection,
}

impl Client {
    pub fn connect(addr: Address) -> Result<Client, io::Error> {
        match addr {
            Address::UnixDomainSocket(path) => {
                let stream = UnixStream::connect(&path)?;
                Ok(Client {
                    conn: Connection::UnixDomainSocket(stream),
                })
            }
            Address::Tcp(addr) => {
                let tcp = TcpStream::connect(addr)?;
                Ok(Client {
                    conn: Connection::Tcp(tcp),
                })
            }
        }
    }
}
