use crate::{
    events::ToServer,
    server::{client, ServerHandle},
};
use std::{fs, path::PathBuf};
use tokio::{io, net::UnixListener};

pub(crate) async fn accept_loop(addr: PathBuf, mut handle: ServerHandle) {
    let res = unix_domain_socket_loop(addr, handle.clone()).await;

    match res {
        Ok(()) => {}
        Err(err) => {
            handle.send(ToServer::FatalError(err)).await;
        }
    }
}

async fn unix_domain_socket_loop(path: PathBuf, handle: ServerHandle) -> Result<(), io::Error> {
    println!("bind");
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
        let (conn, addr) = listen.accept().await?;
        let id = handle.next_id();

        let data = client::unix::ClientInfo {
            id,
            server_handle: handle.clone(),
            conn,
            path: path.to_path_buf(),
        };

        client::unix::spawn_client(data);
    }
}
