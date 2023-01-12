use slotmap::SlotMap;

use self::buffer::{Buffer, BufferId};

pub(crate) mod buffer;

#[derive(Debug, Default)]
pub(crate) struct Buffers {
    buffers: SlotMap<BufferId, Buffer>,
}

impl Buffers {
    pub fn insert(&mut self, buf: Buffer) -> BufferId {
        let id = self.buffers.insert(buf);
        self.buffers[id].id = id;
        id
    }

    pub fn get(&self, id: BufferId) -> Option<&Buffer> {
        self.buffers.get(id)
    }

    pub fn get_mut(&mut self, id: BufferId) -> Option<&mut Buffer> {
        self.buffers.get_mut(id)
    }

    pub fn remove(&mut self, id: BufferId) -> Option<Buffer> {
        self.buffers.remove(id)
    }
}
