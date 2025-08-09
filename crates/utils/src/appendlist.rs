use std::{
    cell::UnsafeCell,
    cmp::min,
    marker::PhantomData,
    mem::{self, MaybeUninit},
    ops::{Index, Range},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Condvar, Mutex,
    },
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

pub struct Appendlist<T> {
    _phantom: PhantomData<T>,
}

impl<T> Appendlist<T> {
    pub fn new() -> (Reader<T>, Writer<T>) {
        let list = Arc::new(List {
            write: AtomicUsize::new(0),
            len: AtomicUsize::new(0),
            buckets: array!(|_| Bucket::default(); BUCKET_COUNT),
        });

        let writer = Writer { list: list.clone() };
        let reader = Reader { list };

        (reader, writer)
    }
}

#[derive(Debug, PartialEq)]
pub enum AppendResult {
    /// Allocated a new block and appended usize bytes to it.
    NewBlock(usize),
    /// Appended usize bytes to an existing block.
    Append(usize),
}

#[derive(Clone, Debug)]
pub struct Writer<T> {
    list: Arc<List<T>>,
}

impl<T: Copy> Writer<T> {
    pub fn append_slice(&self, items: &[T]) -> AppendResult {
        // Using the len here as an index.
        // This is safe because we are the only writer so no one can write to
        // the same location in the meanwhile
        let windex = self.write_index(items.len());
        let loc = BucketLocation::of(windex);
        let bucket = &self.list.buckets[loc.bucket];
        // SAFETY: we are the only writer, and readers will not read after len
        let bucket = unsafe { &mut *bucket.get() };
        let alloc = bucket.is_none();

        if alloc {
            let mut items = Vec::with_capacity(loc.bucket_len);
            for _ in 0..loc.bucket_len {
                items.push(MaybeUninit::uninit());
            }
            *bucket = Some(items.into());
        }

        // SAFETY: we just allocated it if it was not there
        let bucket = bucket.as_mut().unwrap();
        let slice = &mut bucket[loc.pos..];
        let nwrite = min(slice.len(), items.len());
        for i in 0..nwrite {
            slice[i].write(items[i]);
        }

        let idx = self.list.len.fetch_add(nwrite, Ordering::Release);
        debug_assert!(windex == idx, "windex: {} != len: {}", windex, idx);

        if alloc {
            AppendResult::NewBlock(nwrite)
        } else {
            AppendResult::Append(nwrite)
        }
    }
}

impl<T> Writer<T> {
    pub fn append_vec(&self, mut items: Vec<T>) -> AppendResult {
        let windex = self.write_index(items.len());
        let loc = BucketLocation::of(windex);
        let bucket = &self.list.buckets[loc.bucket];
        // SAFETY: we are the only writer, and readers will not read after len
        let bucket = unsafe { &mut *bucket.get() };
        let alloc = bucket.is_none();

        if alloc {
            let mut items = Vec::with_capacity(loc.bucket_len);
            for _ in 0..loc.bucket_len {
                items.push(MaybeUninit::uninit());
            }
            *bucket = Some(items.into());
        }

        // SAFETY: we just allocated it if it was not there
        let bucket = bucket.as_mut().unwrap();
        let slice = &mut bucket[loc.pos..];
        let nwrite = min(slice.len(), items.len());
        for i in (0..nwrite).rev() {
            slice[i].write(items.pop().unwrap());
        }

        let idx = self.list.len.fetch_add(nwrite, Ordering::Release);
        debug_assert!(windex == idx, "windex: {} != len: {}", windex, idx);

        if alloc {
            AppendResult::NewBlock(nwrite)
        } else {
            AppendResult::Append(nwrite)
        }
    }

    pub fn append(&self, item: T) {
        let windex = self.write_index(1);
        let loc = BucketLocation::of(windex);
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
        let slice = &mut bucket[loc.pos..];
        slice[0].write(item);

        let idx = self.list.len.fetch_add(1, Ordering::AcqRel);
        debug_assert!(windex == idx, "windex: {} != len: {}", windex, idx);
    }

    fn write_index(&self, n: usize) -> usize {
        let n = self.list.write.fetch_add(n, Ordering::AcqRel);
        while self.len() < n {
            std::hint::spin_loop();
        }
        n
    }

    pub fn len(&self) -> usize {
        self.list.len.load(Ordering::Acquire)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug)]
pub struct Reader<T> {
    list: Arc<List<T>>,
}

impl<T> Clone for Reader<T> {
    fn clone(&self) -> Self {
        Self {
            list: self.list.clone(),
        }
    }
}

impl<T> Reader<T> {
    pub fn slice(&self, range: Range<usize>) -> &[T] {
        // TODO assert we dont read over bucket boundaries
        let loc = BucketLocation::of(range.start);
        let bucket = {
            let bucket: &UnsafeCell<Option<Box<[MaybeUninit<T>]>>> = &self.list.buckets[loc.bucket];
            let bucket: Option<&Box<[MaybeUninit<T>]>> = unsafe { (*bucket.get()).as_ref() };
            let bucket: &[MaybeUninit<T>] = bucket.unwrap();
            bucket
        };
        let brange = loc.pos..loc.pos + range.len();
        unsafe { mem::transmute(&bucket[brange]) }
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        let loc = BucketLocation::of(idx);
        let bucket = {
            let bucket: &UnsafeCell<Option<Box<[MaybeUninit<T>]>>> = &self.list.buckets[loc.bucket];
            let bucket: Option<&Box<[MaybeUninit<T>]>> = unsafe { (*bucket.get()).as_ref() };
            let bucket: &[MaybeUninit<T>] = bucket.unwrap();
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

impl<T> Index<usize> for Reader<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap()
    }
}

type Bucket<T> = UnsafeCell<Option<Box<[MaybeUninit<T>]>>>;

#[derive(Debug)]
struct List<T> {
    write: AtomicUsize,
    len: AtomicUsize,
    buckets: [Bucket<T>; BUCKET_COUNT],
}

unsafe impl<T> Sync for List<T> {}

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
