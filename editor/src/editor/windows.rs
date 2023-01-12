use std::collections::HashMap;

use crate::server::ClientId;

use self::window::Window;

use super::buffers::buffer::BufferId;

pub(crate) mod window;

#[derive(Debug, Default)]
pub(crate) struct Windows {
    windows: HashMap<ClientId, Window>,
}

impl Windows {
    pub fn new_window(&mut self, id: ClientId, buf: BufferId) {
        self.windows.insert(id, Window::new(buf));
        // TODO return anything?
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
}

