use crate::server::EditorHandle;
use tokio::net::unix::SocketAddr;

pub(crate) async fn accept_loop(addr: SocketAddr, mut handle: EditorHandle) {}
