mod window;

use super::{buffers::BufferId, Map};
use sanedit_server::ClientId;
pub(crate) use window::*;

#[derive(Debug, Default)]
pub(crate) struct Windows {
    windows: Map<ClientId, Window>,
}

impl Windows {
    pub fn new_window(
        &mut self,
        id: ClientId,
        buf: BufferId,
        width: usize,
        height: usize,
        options: WindowConfig,
    ) -> &mut Window {
        let win = Window::new(buf, width, height, options);
        self.windows.insert(id, win);
        self.get_mut(id).unwrap()
    }

    pub fn get(&self, id: ClientId) -> Option<&Window> {
        self.windows.get(&id)
    }

    pub fn get_mut(&mut self, id: ClientId) -> Option<&mut Window> {
        self.windows.get_mut(&id)
    }

    pub fn remove(&mut self, id: ClientId) -> Option<Window> {
        self.windows.remove(&id)
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, ClientId, Window> {
        self.windows.iter()
    }

    pub fn find_clients_with_buf(&self, bid: BufferId) -> Vec<ClientId> {
        self.windows
            .iter()
            .filter(|(_, win)| win.buffer_id() == bid)
            .map(|(cid, _)| *cid)
            .collect()
    }

    pub fn bid(&self, cid: ClientId) -> Option<BufferId> {
        let win = self.windows.get(&cid)?;
        let bid = win.buffer_id();
        Some(bid)
    }
}
