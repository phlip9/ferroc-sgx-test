#![feature(raw_os_error_ty)]
#![feature(ptr_metadata)]
#![feature(allocator_api)]
#![feature(thread_local)]
#![feature(strict_provenance)]
// #![feature(sgx_platform)]

use ferroc::base::{BaseAlloc, Chunk, StaticHandle};
use std::{
    alloc::{AllocError, Layout},
    mem,
    ptr::{self, NonNull},
    sync::atomic::AtomicPtr,
};

mod alloc_expanded;
mod sys;

// Align to x86_64 double cache-line size to prevent false sharing on the atomic
// pointer.
//
// See: <https://docs.rs/crossbeam-utils/0.8.19/src/crossbeam_utils/cache_padded.rs.html#80-87>
#[repr(align(128))]
pub(crate) struct SgxHeap {
    top: AtomicPtr<()>,
}

impl SgxHeap {
    const fn new() -> Self {
        Self {
            top: AtomicPtr::<()>::new(ptr::null_mut()),
        }
    }

    #[inline(always)]
    fn heap_top(&self) -> *mut () {
        use std::sync::atomic::Ordering::{AcqRel, Acquire};

        let base = sys::heap_base();
        match self
            .top
            .compare_exchange(ptr::null_mut(), base, AcqRel, Acquire)
        {
            Ok(_) => base,
            Err(top) => top,
        }
    }

    fn alloc_inner(&self, layout: Layout) -> Option<Chunk<Self>> {
        use std::sync::atomic::Ordering::{AcqRel, Acquire};

        let layout = layout.align_to(mem::align_of::<usize>()).ok()?;
        let mut top = self.heap_top();
        loop {
            let aligned = (top.addr().checked_add(layout.align() - 1))? & !(layout.align() - 1);
            let end = aligned.checked_add(layout.size());
            let end = end.filter(|&end| end < sys::heap_end())?;

            let new = NonNull::new(top.with_addr(aligned))?;
            match self
                .top
                .compare_exchange_weak(top, top.with_addr(end), AcqRel, Acquire)
            {
                // SAFETY: The returned pointer points to an owned memory block of `layout`
                Ok(_) => break Some(unsafe { Chunk::new(new.cast(), layout, StaticHandle) }),
                Err(t) => top = t,
            }
        }
    }
}

unsafe impl Sync for SgxHeap {}

unsafe impl BaseAlloc for SgxHeap {
    const IS_ZEROED: bool = false;

    // A static extant handle. Never deallocated.
    type Handle = StaticHandle;

    type Error = AllocError;

    fn allocate(&self, layout: Layout, _commit: bool) -> Result<Chunk<Self>, Self::Error> {
        self.alloc_inner(layout).ok_or(AllocError)
    }

    unsafe fn deallocate(_chunk: &mut Chunk<Self>) {}
}

mod alloc {
    use super::SgxHeap;

    ferroc::config!(pub SgxAlloc => SgxHeap);
}

#[global_allocator]
static GLOBAL_SGX_ALLOC: crate::alloc::SgxAlloc = crate::alloc::SgxAlloc;

fn main() {
    println!("Hello, world!");
}
