use std::{fs, path::PathBuf};

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
    let listen = match UnixListener::bind(&path) {
        Ok(listen) => listen,
        Err(e) => match e.kind() {
            io::ErrorKind::AddrInUse => {
                fs::remove_file(&path)?;
                UnixListener::bind(&path)?
            }
            _kind => return Err(e),
        },
    };

    loop {
        let (chan, addr) = listen.accept().await?;
        if let Some(path) = addr.as_pathname() {
            let id = handle.next_id();

            let data = ClientInfo {
                id,
                conn: ClientConnection::from_unix_domain_socket(chan, path.to_owned()),
                server_handle: handle.clone(),
            };

            spawn_client(data);
        }
    }
}
