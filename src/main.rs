#![feature(raw_os_error_ty)]
#![feature(ptr_metadata)]
#![feature(allocator_api)]
#![feature(thread_local)]
#![feature(strict_provenance)]
// #![feature(sgx_platform)]

use ferroc::base::{BaseAlloc, Chunk};
use std::{
    alloc::{AllocError, Layout},
    mem,
    ptr::{self, NonNull},
    sync::atomic::AtomicPtr,
};

mod alloc;
mod alloc_expanded;
mod sgx;

pub(crate) struct SgxHeap {
    top: AtomicPtr<()>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SgxHeapHandle;

impl SgxHeap {
    const fn new() -> Self {
        Self {
            top: AtomicPtr::<()>::new(ptr::null_mut()),
        }
    }

    fn alloc_inner(&self, layout: Layout) -> Option<Chunk<Self>> {
        use std::sync::atomic::Ordering::{AcqRel, Acquire};

        let layout = layout.align_to(mem::align_of::<usize>()).ok()?;
        let base = sgx::heap_base();
        let mut top = match self
            .top
            .compare_exchange(ptr::null_mut(), base, AcqRel, Acquire)
        {
            Ok(_) => base,
            Err(top) => top,
        };
        loop {
            let aligned = (top.addr().checked_add(layout.align() - 1))? & !(layout.align() - 1);
            let end = aligned.checked_add(layout.size());
            let end = end.filter(|&end| end < base.addr() + sgx::heap_size())?;

            let new = NonNull::new(top.with_addr(aligned))?;
            match self
                .top
                .compare_exchange_weak(top, top.with_addr(end), AcqRel, Acquire)
            {
                // SAFETY: The returned pointer points to an owned memory block of `layout`
                Ok(_) => break Some(unsafe { Chunk::new(new.cast(), layout, SgxHeapHandle) }),
                Err(t) => top = t,
            }
        }
    }
}

unsafe impl Sync for SgxHeap {}

unsafe impl BaseAlloc for SgxHeap {
    const IS_ZEROED: bool = false;

    type Handle = SgxHeapHandle;

    type Error = AllocError;

    fn allocate(&self, layout: Layout, _commit: bool) -> Result<Chunk<Self>, Self::Error> {
        self.alloc_inner(layout).ok_or(AllocError)
    }

    unsafe fn deallocate(_chunk: &mut Chunk<Self>) {}
}

mod custom {
    use super::SgxHeap;

    ferroc::config!(pub Custom => SgxHeap);
}

#[global_allocator]
static CUSTOM: crate::custom::Custom = crate::custom::Custom;

fn main() {
    println!("Hello, world!");
}
