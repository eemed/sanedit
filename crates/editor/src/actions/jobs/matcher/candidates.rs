use std::{
    cell::UnsafeCell,
    cmp::min,
    io::Write,
    mem::MaybeUninit,
    ops::Range,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

// Idea is to create a array of pointers to blocks of data.
// The blocks of data grow in the powers of two and are allocated as needed.
// The blocks of data are called buckets.
pub(crate) type Candidate = String;

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

pub(crate) struct Candidates;

impl Candidates {
    pub fn new() -> (Reader, Writer) {
        let list = Arc::new(List {
            len: AtomicUsize::new(0),
            buckets: array!(|_| Bucket::default(); BUCKET_COUNT),
        });

        let writer = Writer { list: list.clone() };
        let reader = Reader { list };

        (reader, writer)
    }
}

#[derive(Debug, PartialEq)]
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
    pub fn append_impl(&self, items: Vec<Candidate>) {
        let len = self.list.len.load(Ordering::Relaxed);
        let loc = BucketLocation::of(len);
        let bucket = &self.list.buckets[loc.bucket];
        // SAFETY: we are the only writer, and readers will not read after len
        let bucket = unsafe { &mut *bucket.get() };
        let alloc = bucket.is_none();

        if alloc {
            let mut vec = Vec::with_capacity(loc.bucket_len);
            for _ in 0..loc.bucket_len {
                vec.push(MaybeUninit::uninit());
            }
            *bucket = Some(vec.into());
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

    pub fn append(&self, item: Candidate) {
        let len = self.list.len.load(Ordering::Relaxed);
        let loc = BucketLocation::of(len);
        let bucket = &self.list.buckets[loc.bucket];
        // SAFETY: we are the only writer, and readers will not read after len
        let bucket = unsafe { &mut *bucket.get() };
        let alloc = bucket.is_none();

        if alloc {
            let mut vec = Vec::with_capacity(loc.bucket_len);
            for _ in 0..loc.bucket_len {
                vec.push(MaybeUninit::uninit());
            }
            *bucket = Some(vec.into());
        }

        // SAFETY: we just allocated it if it was not there
        let bucket = bucket.as_mut().unwrap();
        let mut slice = &mut bucket[loc.pos..];
        // let nwrite = min(slice.len(), bytes.len());
        slice[0] = MaybeUninit::new(item);

        self.list.len.store(len + 1, Ordering::Release);
    }

    pub fn len(&self) -> usize {
        self.list.len.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Reader {
    list: Arc<List>,
}

impl Reader {
    pub fn slice(&self, range: Range<usize>) -> &[Candidate] {
        // TODO assert we dont read past len
        let loc = BucketLocation::of(range.start);
        let bucket = {
            let bucket: &UnsafeCell<Option<Box<[Candidate]>>> = &self.list.buckets[loc.bucket];
            let bucket: Option<&Box<[Candidate]>> = unsafe { (*bucket.get()).as_ref() };
            let bucket: &Box<[Candidate]> = bucket.unwrap();
            bucket
        };
        let brange = loc.pos..loc.pos + range.len();
        &bucket[brange]
    }

    pub fn len(&self) -> usize {
        self.list.len.load(Ordering::Relaxed)
    }
}

type Bucket = UnsafeCell<Option<Box<[MaybeUninit<Candidate>]>>>;

#[derive(Debug)]
struct List {
    len: AtomicUsize,
    buckets: [Bucket; BUCKET_COUNT],
}

unsafe impl Sync for List {}

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
        debug_assert!(pos < usize::MAX - BUCKET_START_POS);

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
