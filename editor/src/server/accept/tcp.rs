use crate::server::ServerHandle;
use tokio::net::unix::SocketAddr;

pub(crate) async fn accept_loop(addr: SocketAddr, mut handle: ServerHandle) {}
