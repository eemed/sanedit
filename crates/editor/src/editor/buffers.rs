mod buffer;

use std::path::Path;

use sanedit_utils::idmap::IdMap;

pub(crate) use self::buffer::{
    Buffer, BufferId, BufferRange, Filetype, SnapshotData, SnapshotId, SortedRanges,
};

#[derive(Debug, Default)]
pub(crate) struct Buffers {
    buffers: IdMap<BufferId, Buffer>,
}

impl Buffers {
    pub fn insert(&mut self, buf: Buffer) -> BufferId {
        let id = self.buffers.insert(buf);
        self.buffers[id].id = id;
        id
    }

    pub fn get(&self, bid: BufferId) -> Option<&Buffer> {
        self.buffers.get(&bid)
    }

    pub fn get_mut(&mut self, bid: BufferId) -> Option<&mut Buffer> {
        self.buffers.get_mut(&bid)
    }

    pub fn remove(&mut self, bid: BufferId) -> Option<Buffer> {
        self.buffers.remove(&bid)
    }

    pub fn iter(&self) -> sanedit_utils::idmap::Iter<BufferId, Buffer> {
        self.buffers.iter()
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
