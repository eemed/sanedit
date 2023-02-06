use crate::{
    events::ToEditor,
    server::{client, EditorHandle},
};
use std::{fs, path::PathBuf};
use tokio::{io, net::UnixListener};

pub(crate) async fn accept_loop(addr: PathBuf, mut handle: EditorHandle) {
    let res = unix_domain_socket_loop(addr, handle.clone()).await;

    match res {
        Ok(()) => {}
        Err(err) => {
            handle.send(ToEditor::FatalError(err)).await;
        }
    }
}

async fn unix_domain_socket_loop(path: PathBuf, handle: EditorHandle) -> Result<(), io::Error> {
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
