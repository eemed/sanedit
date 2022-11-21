mod buffer;
mod window;

pub(crate) use buffer::*;
use slotmap::SlotMap;
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
    buffers: SlotMap<BufferId, Buffer>,
}

impl Editor {
    fn new() -> Editor {
        Editor {
            clients: HashMap::new(),
            windows: HashMap::new(),
            buffers: SlotMap::with_key(),
        }
    }
}

/// Execute editor logic, getting each message from the passed receiver.
/// Editor uses client handles to communicate to clients. Client handles are
/// sent using the provided reciver.
pub(crate) async fn main_loop(mut recv: Receiver<ToServer>) -> Result<(), io::Error> {
    println!("Main loop");
    let mut editor = Editor::new();

    while let Some(msg) = recv.recv().await {
        // TODO do editor logic
        println!("Server got: {:?}", msg);
    }

    println!("Main loop exiting");
    Ok(())
}
