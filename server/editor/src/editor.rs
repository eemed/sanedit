mod buffer;
mod window;

pub(crate) use buffer::*;
pub(crate) use window::*;

use std::collections::HashMap;

use tokio::io;
use tokio::sync::mpsc::Receiver;

use crate::events::ToServer;
use crate::server::ClientHandle;
use crate::server::ClientId;

pub(crate) struct Editor {
    clients: HashMap<ClientId, ClientHandle>,
    windows: HashMap<ClientId, Window>,
    // buffers: SlotMap<BufferId, Buffer>,
}

impl Editor {
    fn new() -> Editor {
        todo!()
    }
}

/// Execute editor logic, getting each message from the passed receiver.
/// Editor uses client handles to communicate to clients. Client handles are
/// sent using the provided reciver.
pub(crate) async fn main_loop(mut recv: Receiver<ToServer>) -> Result<(), io::Error> {
    let mut editor = Editor::new();

    while let Some(event) = recv.recv().await {
        // TODO do editor logic
    }

    Ok(())
}
