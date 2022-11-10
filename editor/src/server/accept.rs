use std::path::PathBuf;

use tokio::{
    io,
    net::{unix::SocketAddr, TcpListener, UnixListener},
};

use crate::events::ToServer;

use super::{spawn_client, ClientConnection, ClientInfo, ListenAddr, ServerHandle};

pub async fn spawn_accept(addr: ListenAddr, mut handle: ServerHandle) {
    let res = match addr {
        ListenAddr::UnixDomainSocket(path) => unix_domain_socket_loop(path, handle.clone()).await,
        ListenAddr::Tcp(addr) => todo!(),
    };

    match res {
        Ok(()) => {}
        Err(err) => {
            handle.send(ToServer::FatalError(err)).await;
        }
    }
}

async fn unix_domain_socket_loop(path: PathBuf, mut handle: ServerHandle) -> Result<(), io::Error> {
    let listen = UnixListener::bind(path)?;

    loop {
        let (chan, addr) = listen.accept().await?;
        let path = addr
            .as_pathname()
            .expect("unix domain socket listener got unnamed client")
            .to_path_buf();

        let id = handle.next_id();

        let data = ClientInfo {
            id,
            conn: ClientConnection::UnixDomainSocket { path, chan },
            server_handle: handle.clone(),
        };

        spawn_client(data);
    }
}
