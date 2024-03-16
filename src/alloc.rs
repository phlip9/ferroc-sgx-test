use ferroc::base::{BaseAlloc, StaticHandle};
use std::{
    alloc::{AllocError, Layout},
    mem,
    ptr::{self, NonNull},
    sync::atomic::AtomicPtr,
};

use crate::sys;

/// Aligns the inner `T` to x86_64 double cache-line size. Used to prevent
/// [false sharing]. Based on [`crossbeam_utils::CachePadded`].
///
/// [false sharing]: https://www.wikiwand.com/en/False_sharing
/// [`crossbeam_utils::CachePadded`]: https://docs.rs/crossbeam-utils/0.8.19/crossbeam_utils/struct.CachePadded.html
#[repr(align(128))]
struct CachePadded<T>(T);

// `SgxHeapAlloc` is a full-featured allocator (e.g. can actually free memory).
ferroc::config!(pub SgxHeapAlloc => SgxHeapBump);

/// A bump allocator over the fixed SGX heap, used for allocating larger extants
/// by the fullf-featured [`SgxHeapAlloc`] allocator.
struct SgxHeapBump {
    top: CachePadded<AtomicPtr<()>>,
}

impl SgxHeapBump {
    #[inline(always)]
    const fn new() -> Self {
        Self {
            top: CachePadded(AtomicPtr::<()>::new(ptr::null_mut())),
        }
    }

    #[inline(always)]
    fn heap_top(&self) -> *mut () {
        use std::sync::atomic::Ordering::{AcqRel, Acquire};

        let base = sys::heap_base();
        match self
            .top
            .0
            .compare_exchange(ptr::null_mut(), base, AcqRel, Acquire)
        {
            Ok(_) => base,
            Err(top) => top,
        }
    }

    #[inline(always)]
    fn alloc_inner(&self, layout: Layout) -> Option<ferroc::base::Chunk<Self>> {
        use std::sync::atomic::Ordering::{AcqRel, Acquire};

        let layout = layout.align_to(mem::align_of::<usize>()).ok()?;
        let mut top = self.heap_top();
        loop {
            let aligned = (top.addr().checked_add(layout.align() - 1))? & !(layout.align() - 1);
            let end = aligned.checked_add(layout.size());
            let end = end.filter(|&end| end < sys::heap_end().addr())?;

            let new = NonNull::new(top.with_addr(aligned))?;
            match self
                .top
                .0
                .compare_exchange_weak(top, top.with_addr(end), AcqRel, Acquire)
            {
                // SAFETY: The returned pointer points to an owned memory block of `layout`
                Ok(_) => {
                    break Some(unsafe {
                        ferroc::base::Chunk::new(new.cast(), layout, StaticHandle)
                    })
                }
                Err(t) => top = t,
            }
        }
    }
}

unsafe impl BaseAlloc for SgxHeapBump {
    const IS_ZEROED: bool = false;

    // A static extant handle. Never deallocated.
    type Handle = StaticHandle;

    type Error = AllocError;

    fn allocate(
        &self,
        layout: Layout,
        _commit: bool,
    ) -> Result<ferroc::base::Chunk<Self>, Self::Error> {
        self.alloc_inner(layout).ok_or(AllocError)
    }

    unsafe fn deallocate(_chunk: &mut ferroc::base::Chunk<Self>) {}
}
