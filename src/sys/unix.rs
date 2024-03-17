use std::cell::UnsafeCell;

const HEAP_SIZE: usize = 4 * (1 << 20); // 4 MiB

static HEAP: Heap<HEAP_SIZE> = Heap::<HEAP_SIZE>::new();

// Page align
#[repr(align(4096))]
struct Heap<const HEAP_SIZE: usize> {
    inner: UnsafeCell<[u8; HEAP_SIZE]>,
}

impl<const HEAP_SIZE: usize> Heap<HEAP_SIZE> {
    const fn new() -> Self {
        Self {
            inner: UnsafeCell::new([0u8; HEAP_SIZE]),
        }
    }

    #[inline(always)]
    fn base_ptr(&self) -> *mut () {
        self.inner.get().cast()
    }

    #[inline(always)]
    const fn size(&self) -> usize {
        HEAP_SIZE
    }
}

unsafe impl<const HEAP_SIZE: usize> Sync for Heap<HEAP_SIZE> {}

#[inline(always)]
pub(crate) fn heap_base_ptr() -> *mut () {
    HEAP.base_ptr()
}

#[inline(always)]
pub(crate) fn heap_size() -> usize {
    HEAP.size()
}
