use std::{
    cell::{RefCell, UnsafeCell},
    cmp::min,
    io::Write,
    ops::Range,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use super::ByteSlice;

// Idea is to create a array of pointers to blocks of data.
// The blocks of data grow in the powers of two and are allocated as needed.
// The blocks of data are called buckets.

// To ensure we dont need to split content
// alot in the beginning, start straight from bucket 14 => 2^14 => 16kb bucket
const BUCKET_START: usize = 14;
const BUCKET_START_POS: usize = 1 << BUCKET_START;
const BUCKET_COUNT: usize = usize::BITS as usize - BUCKET_START;

macro_rules! array {
    (
    $closure:expr; $N:expr
) => {{
        use ::core::{
            mem::{forget, MaybeUninit},
            ptr, slice,
        };

        const N: usize = $N;

        #[inline(always)]
        fn gen_array<T>(mut closure: impl FnMut(usize) -> T) -> [T; N] {
            unsafe {
                let mut array = MaybeUninit::uninit();

                struct PartialRawSlice<T> {
                    ptr: *mut T,
                    len: usize,
                }

                impl<T> Drop for PartialRawSlice<T> {
                    fn drop(self: &'_ mut Self) {
                        unsafe { ptr::drop_in_place(slice::from_raw_parts_mut(self.ptr, self.len)) }
                    }
                }

                let mut raw_slice = PartialRawSlice {
                    ptr: array.as_mut_ptr() as *mut T,
                    len: 0,
                };

                (0..N).for_each(|i| {
                    ptr::write(raw_slice.ptr.add(i), closure(i));
                    raw_slice.len += 1;
                });

                forget(raw_slice);
                array.assume_init()
            }
        }

        gen_array($closure)
    }};
}

#[derive(Debug)]
pub(crate) struct AddBuffer {
    writer: Writer,
    reader: Reader,
}

impl AddBuffer {
    pub fn new() -> AddBuffer {
        let list = Arc::new(List {
            len: AtomicUsize::new(0),
            buckets: array!(|_| Bucket::default(); BUCKET_COUNT),
        });

        let writer = Writer { list: list.clone() };
        let reader = Reader { list };

        AddBuffer { writer, reader }
    }

    pub fn len(&self) -> usize {
        self.reader.list.len.load(Ordering::Relaxed)
    }

    /// Append to add buffer.
    /// This will only append the amount we can guarantee are contiguous.
    /// This will ensure you can slice the buffer from these points later using
    /// slice, and no copying will be done.
    ///
    /// This is used to create separate pieces in the tree when the data cannot be
    /// contiguous in memory.
    pub fn append(&self, bytes: &[u8]) -> AppendResult {
        self.writer.append(bytes)
    }

    pub fn slice<'a>(&'a self, range: Range<usize>) -> ByteSlice<'a> {
        let bytes = self.reader.slice(range);
        ByteSlice::Borrowed(bytes)
    }

    pub fn reader(&self) -> Reader {
        todo!()
    }
}

pub(crate) enum AppendResult {
    /// Allocated a new block and appended usize bytes to it.
    NewBlock(usize),
    /// Appended usize bytes to an existing block.
    Append(usize),
}

#[derive(Debug)]
pub(crate) struct Writer {
    list: Arc<List>,
}

impl Writer {
    pub fn append(&self, bytes: &[u8]) -> AppendResult {
        let len = self.list.len.load(Ordering::Relaxed);
        let loc = BucketLocation::of(len);
        let bucket = &self.list.buckets[loc.bucket];
        // SAFETY: we are the only writer, and readers will not read after len
        let bucket = unsafe { &mut *bucket.get() };
        let alloc = bucket.is_none();

        if alloc {
            *bucket = Some(vec![0u8; loc.bucket_len].into());
        }

        // SAFETY: we just allocated it if it was not there
        let bucket = bucket.as_mut().unwrap();
        let mut slice = &mut bucket[loc.pos..];
        let nwrite = min(slice.len(), bytes.len());
        slice
            .write_all(&bytes[..nwrite])
            .expect("Failed to write bytes to bucket");

        self.list.len.store(len + nwrite, Ordering::Release);

        if alloc {
            AppendResult::NewBlock(nwrite)
        } else {
            AppendResult::Append(nwrite)
        }
    }
}

#[derive(Debug)]
pub(crate) struct Reader {
    list: Arc<List>,
}

impl Reader {
    pub fn slice(&self, range: Range<usize>) -> &[u8] {
        // TODO assert we dont read past len
        let loc = BucketLocation::of(range.start);
        let bucket = &self.list.buckets[loc.bucket];
        let bucket = unsafe { (*bucket.get()).as_ref() };
        let bucket = bucket.unwrap();
        let brange = loc.pos..loc.pos + range.len();
        &bucket[brange]
    }
}

type Bucket = UnsafeCell<Option<Box<[u8]>>>;

#[derive(Debug)]
struct List {
    len: AtomicUsize,
    buckets: [Bucket; BUCKET_COUNT],
}

#[derive(Debug, PartialEq)]
struct BucketLocation {
    /// Index of the bucket in the list
    bucket: usize,

    /// Length of the data in the bucket
    bucket_len: usize,

    /// Position in the bucket
    pos: usize,
}

impl BucketLocation {
    pub fn of(pos: usize) -> BucketLocation {
        let pos = pos + BUCKET_START_POS;
        let bucket = (usize::BITS - pos.leading_zeros()) as usize;
        let bucket_len = 1 << bucket.saturating_sub(1);
        let pos = if pos == 0 { 0 } else { pos ^ bucket_len };

        // Fix bucket index to take account our starting bucket
        let bucket = bucket - BUCKET_START - 1;

        BucketLocation {
            bucket,
            bucket_len,
            pos,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn location() {
        assert_eq!(
            BucketLocation {
                bucket: 0,
                bucket_len: 1 << BUCKET_START,
                pos: 0
            },
            BucketLocation::of(0)
        );

        assert_eq!(
            BucketLocation {
                bucket: 0,
                bucket_len: 1 << BUCKET_START,
                pos: 200
            },
            BucketLocation::of(200)
        );

        assert_eq!(
            BucketLocation {
                bucket: 1,
                bucket_len: 1 << (BUCKET_START + 1),
                pos: 0
            },
            BucketLocation::of(16384)
        );

        assert_eq!(
            BucketLocation {
                bucket: 1,
                bucket_len: 1 << (BUCKET_START + 1),
                pos: 1
            },
            BucketLocation::of(16385)
        );
    }

    #[test]
    fn append() {
        let add = AddBuffer::new();
        let bytes = b"abba";
        add.append(bytes);
        println!("LEN {}", add.len());
    }
}
