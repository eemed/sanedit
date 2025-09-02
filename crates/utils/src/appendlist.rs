use std::{
    cell::UnsafeCell,
    cmp::min,
    mem::{self, MaybeUninit},
    ops::{Index, Range},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, OnceLock,
    },
    thread,
    time::Duration,
};

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

#[derive(Debug, PartialEq)]
pub enum AppendResult {
    /// Allocated a new block and appended usize bytes to it.
    NewBlock(usize),
    /// Appended usize bytes to an existing block.
    Append(usize),
}

type Bucket<T> = OnceLock<UnsafeCell<Box<[MaybeUninit<T>]>>>;

#[derive(Debug)]
struct List<T> {
    write: AtomicUsize,
    len: AtomicUsize,
    buckets: [Bucket<T>; BUCKET_COUNT],
}

unsafe impl<T: Send> Send for List<T> {}
unsafe impl<T: Sync + Send> Sync for List<T> {}

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
    fn of(pos: usize) -> BucketLocation {
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

#[derive(Debug)]
pub struct Appendlist<T> {
    list: Arc<List<T>>,
}

impl<T> Clone for Appendlist<T> {
    fn clone(&self) -> Self {
        let list = Arc::clone(&self.list);
        Appendlist { list }
    }
}

unsafe impl<T: Send> Send for Appendlist<T> {}
unsafe impl<T: Sync + Send> Sync for Appendlist<T> {}

impl<T> Appendlist<T> {
    pub fn new() -> Appendlist<T> {
        let list = Arc::new(List {
            write: AtomicUsize::new(0),
            len: AtomicUsize::new(0),
            buckets: array!(|_| Bucket::default(); BUCKET_COUNT),
        });

        Appendlist { list }
    }

    pub fn split() -> (Reader<T>, Writer<T>) {
        let list = Self::new();
        let writer = Writer { list: list.clone() };
        let reader = Reader { list };
        (reader, writer)
    }
}

impl<T> Appendlist<T> {
    fn write_index(&self, n: usize) -> usize {
        self.list.write.fetch_add(n, Ordering::AcqRel)
    }

    fn inc_len(&self, write_index: usize, written: usize) {
        let mut backoff = 1;
        while self
            .list
            .len
            .compare_exchange(
                write_index,
                write_index + written,
                Ordering::Release,
                Ordering::Relaxed,
            )
            .is_err()
        {
            thread::sleep(Duration::from_micros(backoff));
            backoff = (backoff * 2).min(100); // Cap the delay
        }
    }

    pub fn append(&self, item: T) {
        let write_index = self.write_index(1);
        let loc = BucketLocation::of(write_index);
        let bucket = self.list.buckets[loc.bucket].get_or_init(|| {
            let mut vec = Vec::with_capacity(loc.bucket_len);
            for _ in 0..loc.bucket_len {
                vec.push(MaybeUninit::uninit());
            }
            UnsafeCell::new(vec.into())
        });
        let bucket = unsafe { &mut *bucket.get() };
        let bucket = bucket.as_mut();
        let slice = &mut bucket[loc.pos..];
        slice[0].write(item);

        self.inc_len(write_index, 1);
    }

    pub fn slice(&self, range: Range<usize>) -> &[T] {
        let loc = BucketLocation::of(range.start);
        let end = BucketLocation::of(range.end);
        debug_assert!(
            (loc.bucket == end.bucket) || (loc.bucket + 1 == end.bucket  && end.pos == 0),
            "Slicing from different buckets slice: {range:?}, start: {loc:?}, end: {end:?}"
        );

        let bucket = {
            let bucket: &UnsafeCell<Box<[MaybeUninit<T>]>> =
                &self.list.buckets[loc.bucket].get().unwrap();
            let bucket: &[MaybeUninit<T>] = unsafe { (*bucket.get()).as_ref() };
            bucket
        };
        let brange = loc.pos..loc.pos + range.len();
        unsafe { mem::transmute(&bucket[brange]) }
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        let loc = BucketLocation::of(idx);
        let bucket = {
            let bucket: &UnsafeCell<Box<[MaybeUninit<T>]>> =
                &self.list.buckets[loc.bucket].get().unwrap();
            let bucket: &[MaybeUninit<T>] = unsafe { (*bucket.get()).as_ref() };
            bucket
        };
        unsafe { mem::transmute(&bucket[loc.pos]) }
    }

    pub fn len(&self) -> usize {
        self.list.len.load(Ordering::Acquire)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T> Index<usize> for Appendlist<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap()
    }
}

#[derive(Debug)]
pub struct Writer<T> {
    list: Appendlist<T>,
}

unsafe impl<T: Send> Send for Writer<T> {}
unsafe impl<T: Sync + Send> Sync for Writer<T> {}

impl<T> Writer<T> {
    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn append_vec(&self, mut items: Vec<T>) -> AppendResult {
        let pos = self.list.len();
        let loc = BucketLocation::of(pos);
        let mut alloc = false;
        let bucket = self.list.list.buckets[loc.bucket].get_or_init(|| {
            alloc = true;
            let mut vec = Vec::with_capacity(loc.bucket_len);
            for _ in 0..loc.bucket_len {
                vec.push(MaybeUninit::uninit());
            }
            UnsafeCell::new(vec.into())
        });

        // SAFETY: we just allocated it if it was not there
        let bucket = unsafe { &mut *bucket.get() };
        let bucket = bucket.as_mut();
        let slice = &mut bucket[loc.pos..];
        let nwrite = min(slice.len(), items.len());
        for i in (0..nwrite).rev() {
            slice[i].write(items.pop().unwrap());
        }

        self.list.list.len.store(pos + nwrite, Ordering::Release);

        if alloc {
            AppendResult::NewBlock(nwrite)
        } else {
            AppendResult::Append(nwrite)
        }
    }
}

impl<T: Copy> Writer<T> {
    pub fn append_slice(&self, items: &[T]) -> AppendResult {
        let pos = self.list.len();
        let loc = BucketLocation::of(pos);
        let mut alloc = false;
        let bucket = self.list.list.buckets[loc.bucket].get_or_init(|| {
            alloc = true;
            let mut vec = Vec::with_capacity(loc.bucket_len);
            for _ in 0..loc.bucket_len {
                vec.push(MaybeUninit::uninit());
            }
            UnsafeCell::new(vec.into())
        });

        // SAFETY: we just allocated it if it was not there
        let bucket = unsafe { &mut *bucket.get() };
        let bucket = bucket.as_mut();
        let slice = &mut bucket[loc.pos..];
        let nwrite = min(slice.len(), items.len());
        for i in 0..nwrite {
            slice[i].write(items[i]);
        }

        self.list.list.len.store(pos + nwrite, Ordering::Release);

        if alloc {
            AppendResult::NewBlock(nwrite)
        } else {
            AppendResult::Append(nwrite)
        }
    }
}

#[derive(Debug, Clone)]
pub struct Reader<T> {
    list: Appendlist<T>,
}

unsafe impl<T: Send> Send for Reader<T> {}
unsafe impl<T: Sync + Send> Sync for Reader<T> {}

impl<T> Reader<T> {
    pub fn slice(&self, range: Range<usize>) -> &[T] {
        self.list.slice(range)
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        self.list.get(idx)
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }
}
