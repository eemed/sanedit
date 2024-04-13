mod window;

use super::buffers::BufferId;
use crate::server::ClientId;
use rustc_hash::FxHashMap;
pub(crate) use window::*;

#[derive(Debug, Default)]
pub(crate) struct Windows {
    windows: FxHashMap<ClientId, Window>,
}

impl Windows {
    pub fn new_window(
        &mut self,
        id: ClientId,
        buf: BufferId,
        width: usize,
        height: usize,
    ) -> &mut Window {
        self.windows.insert(id, Window::new(buf, width, height));
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

    pub fn iter(&self) -> std::collections::hash_map::Iter<ClientId, Window> {
        self.windows.iter()
    }

    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<ClientId, Window> {
        self.windows.iter_mut()
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
