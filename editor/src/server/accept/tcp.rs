use std::sync::Arc;

use crate::server::EditorHandle;
use tokio::{net::unix::SocketAddr, sync::Notify};

pub(crate) async fn accept_loop(addr: SocketAddr, mut handle: EditorHandle, notify: Arc<Notify>) {}
