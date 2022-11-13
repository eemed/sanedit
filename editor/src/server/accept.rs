use std::path::PathBuf;

use tokio::{io, net::UnixListener};

use crate::events::ToServer;

use super::{spawn_client, ClientConnection, ClientInfo, ListenAddr, ServerHandle};

pub(crate) async fn accept_loop(addr: ListenAddr, mut handle: ServerHandle) {
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

async fn unix_domain_socket_loop(path: PathBuf, handle: ServerHandle) -> Result<(), io::Error> {
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
            conn: ClientConnection::from_unix_domain_socket(chan, path),
            server_handle: handle.clone(),
        };

        spawn_client(data);
    }
}
