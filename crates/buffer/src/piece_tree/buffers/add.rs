use std::{
    cell::UnsafeCell,
    cmp::min,
    io::Write,
    ops::Range,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use crate::piece_tree::FILE_BACKED_MAX_PIECE_SIZE;

use super::ByteSlice;

pub(crate) const LIST_NODE_DATA_SIZE: usize = FILE_BACKED_MAX_PIECE_SIZE;

#[derive(Debug)]
pub(crate) struct AddBuffer {
    writer: Writer,
    reader: Reader,
}

impl AddBuffer {
    /// Append to add buffer.
    /// This will only append the amount we can guarantee are contiguous.
    /// This will ensure you can slice the buffer from these points later using
    /// slice, and no copying will be done.
    ///
    /// This is used to create separate pieces in the tree when the data cannot be
    /// contiguous in memory.
    pub fn append_contiguous(&self, bytes: &[u8]) -> AppendResult {
        self.writer.append_contiguous(bytes)
    }

    pub fn slice_contiguous<'a>(&'a self, range: Range<usize>) -> ByteSlice<'a> {
        todo!()
    }
}

#[derive(Debug)]
pub(crate) struct Writer {
    list: Arc<List>,
    nodes: usize,
    tail: *const Node,
}

pub(crate) enum AppendResult {
    /// Allocated a new block and appended usize bytes to it.
    NewBlock(usize),
    /// Appended usize bytes to an existing block.
    Append(usize),
}

impl Writer {
    pub fn append(&self, bytes: &[u8]) -> AppendResult {
        let mut cap = self.nodes * LIST_NODE_DATA_SIZE;
        let len = self.list.len.load(Ordering::Relaxed);
        let mut allocated = false;
        if cap <= len {
            self.allocate_next();
            cap = self.nodes * LIST_NODE_DATA_SIZE;
            allocated = true;
        }

        let n = min(cap - len, bytes.len());
        let tpos = len % LIST_NODE_DATA_SIZE;
        let tail = unsafe { &mut *self.tail };
        let data = &mut tail.data.as_mut()[tpos..];
        data.write_all(&bytes[..n])
            .expect("Failed to write bytes to node buffer");

        if allocated {
            AppendResult::NewBlock(n)
        } else {
            AppendResult::Append(n)
        }
    }

    fn allocate_next(&self) {
        let node = Arc::new(Node {
            next: None,
            data: vec![0u8; LIST_NODE_DATA_SIZE].into_boxed_slice(),
        });
        let tail = unsafe { &mut *self.tail };
        self.tail = Arc::as_ptr(&node);
        tail.next = Some(node);
        self.nodes += 1;
    }
}

#[derive(Debug)]
pub(crate) struct Reader {
    list: Arc<List>,
}

impl Reader {
    pub fn slice(&self, range: Range<usize>) -> &[u8] {
        let mut nnode = range.start / LIST_NODE_DATA_SIZE;
        let mut node = self.list.head.as_ref();

        for _ in 0..nnode {
            node = node.next.unwrap().as_ref();
        }

        let nrange = range.start % LIST_NODE_DATA_SIZE..range.end % LIST_NODE_DATA_SIZE;
        &node.data[nrange]
    }
}

#[derive(Debug)]
struct List {
    len: AtomicUsize,
    head: Arc<Node>,
}

#[derive(Debug)]
struct Node {
    next: Option<Arc<Node>>,
    data: Box<[u8]>,
}
