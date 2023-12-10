mod buffer;

use std::path::Path;

use slotmap::SlotMap;

pub(crate) use self::buffer::{
    Buffer, BufferId, Change, ChangeKind, SnapshotData, SnapshotId, SortedRanges,
};

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

    /// Find buffer with a save path
    pub fn find(&self, path: impl AsRef<Path>) -> Option<BufferId> {
        let path = path.as_ref();

        for (id, buf) in self.buffers.iter() {
            if let Some(bpath) = buf.path() {
                if bpath == path {
                    return Some(id);
                }
            }
        }

        None
    }
}
