use crate::{
    events::ToEditor,
    server::{client, EditorHandle},
};
use std::{fs, path::PathBuf, sync::Arc};
use tokio::{io, net::UnixListener, sync::Notify};

pub(crate) async fn accept_loop(addr: PathBuf, mut handle: EditorHandle, notify: Arc<Notify>) {
    let h = handle.clone();
    let res = unix_domain_socket_loop(addr, h, notify).await;

    match res {
        Ok(()) => {}
        Err(err) => {
            log::error!("unix domain socket accept loop failure: {}", err);
            handle.send(ToEditor::FatalError(err));
        }
    }
}

async fn unix_domain_socket_loop(
    path: PathBuf,
    handle: EditorHandle,
    notify: Arc<Notify>,
) -> Result<(), io::Error> {
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

    notify.notify_one();

    loop {
        let (conn, _addr) = listen.accept().await?;
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
