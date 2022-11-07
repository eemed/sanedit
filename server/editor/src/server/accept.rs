use tokio::{net::{TcpListener, unix::SocketAddr}, io};

use crate::events::ToServer;

use super::ServerHandle;

async fn spawn_accept(bind: SocketAddr, mut handle: ServerHandle) {
    let res = accept_loop(bind, handle.clone()).await;
    match res {
        Ok(()) => {}
        Err(err) => {
            handle.send(ToServer::FatalError(err)).await;
        }
    }
}

async fn accept_loop(bind: SocketAddr, mut handle: ServerHandle) -> Result<(), io::Error> {
    let listen = TcpListener::bind(bind).await?;

    loop {
        let (tcp, ip) = listen.accept().await?;

        let id = handle.next_id();

        let data = ClientInfo {
            ip,
            id,
            tcp,
            handle: handle.clone(),
        };

        spawn_client(data);
    }
}
