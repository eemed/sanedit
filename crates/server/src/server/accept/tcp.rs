use std::sync::Arc;

use crate::server::EditorHandle;
use tokio::{net::unix::SocketAddr, sync::Notify};

pub(crate) async fn accept_loop(_addr: SocketAddr, _handle: EditorHandle, _notify: Arc<Notify>) {}
