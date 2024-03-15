#![allow(dead_code)]

use ferroc::base::Static;
const HEADER_CAP: usize = 4096;
static STATIC: Static<HEADER_CAP> = Static::new();
mod thread {
    use super::{Heap, THREAD_LOCALS};
    use core::{cell::Cell, num::NonZeroU64, pin::Pin};

    #[thread_local]
    static HEAP: Cell<Pin<&Heap>> = Cell::new(THREAD_LOCALS.empty_heap());

    #[inline(always)]
    pub fn with<T>(f: impl FnOnce(&Heap) -> T) -> T {
        f(&HEAP.get())
    }

    #[inline(always)]
    pub fn with_lazy<T, F>(f: F) -> T
    where
        F: for<'a> FnOnce(&'a Heap<'static, 'static>, fn() -> &'a Heap<'static, 'static>) -> T,
    {
        fn fallback<'a>() -> &'a Heap<'static, 'static> {
            let (heap, id) = Pin::static_ref(&THREAD_LOCALS).assign();
            unsafe { init(id) };
            HEAP.set(heap);
            Pin::get_ref(heap)
        }
        f(Pin::get_ref(HEAP.get()), fallback)
    }

    unsafe fn init(_id: NonZeroU64) {}
}

/// The chunk type of the `&'static Static<HEADER_CAP>` backend.
/// See [`Chunk`]($crate::base::Chunk) for more information.
pub type Chunk = ::ferroc::base::Chunk<&'static Static<HEADER_CAP>>;

/// The heap type of the `&'static Static<HEADER_CAP>` backend.
/// See [`Heap`]($crate::heap::Heap) for more information.
pub type Heap<'arena, 'cx> = ::ferroc::heap::Heap<'arena, 'cx, &'static Static<HEADER_CAP>>;

/// The arena collection type of the `&'static Static<HEADER_CAP>` backend.
/// See [`Arenas`]($crate::arena::Arenas) for more information.
pub type Arenas = ::ferroc::arena::Arenas<&'static Static<HEADER_CAP>>;

/// The error type of the `&'static Static<HEADER_CAP>` backend.
/// See [`Error`]($crate::arena::Error) for more information.
pub type Error = ::ferroc::arena::Error<&'static Static<HEADER_CAP>>;

type ThreadLocal<'arena> = ::ferroc::heap::ThreadLocal<'arena, &'static Static<HEADER_CAP>>;
type AllocateOptions<F> = ::ferroc::heap::AllocateOptions<F>;
static ARENAS: Arenas = Arenas::new(&STATIC);
static THREAD_LOCALS: ThreadLocal<'static> = ThreadLocal::new(&ARENAS);

/// The configured allocator backed by `&'static Static<HEADER_CAP>`.
/// This allocator is the interface of a global instance of arenas
/// and thread-local contexts and heaps. It forwards most of the
/// function calls to the actual implementation of them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Custom;

impl Custom {
    /// Retrieves the base allocator of this configured memory allocator.
    ///
    /// This function forwards the call to [`Arenas::base`].
    #[inline]
    pub fn base(&self) -> &&'static Static<HEADER_CAP> {
        ARENAS.base()
    }

    /// Manages another chunk previously allocated by an instance of its base
    /// allocator.
    ///
    /// This function creates a new arena from the chunk and push it to the
    /// allocator for further allocation, extending the heap's overall
    /// capacity.
    ///
    /// This function forwards the call to [`Arenas::manage`].
    ///
    /// # Panics
    ///
    /// This function panics if the alignment of the chunk is less than
    ///[`SLAB_SIZE`]($crate::arena::SLAB_SIZE).
    #[inline]
    pub fn manage(&self, chunk: Chunk) -> Result<(), Error> {
        ARENAS.manage(chunk)
    }

    /// Clean up some garbage data of the current heap immediately.
    ///
    /// In short, due to implementation details, the free list (i.e. the popper
    /// of allocation) and the deallocation list (i.e. the pusher of
    /// deallocation) of a [`Heap`] are 2 distinct lists.
    ///
    /// Those 2 lists will be swapped if the former becomes empty during the
    /// allocation process, which is precisely what this function does.
    ///
    /// This function forwards the call to [`Heap::collect`].
    #[inline]
    pub fn collect(&self, force: bool) {
        thread::with(|heap| heap.collect(force));
    }

    /// Allocate a memory block of `layout` from the current heap.
    ///
    /// The allocation can be deallocated by any instance of this configured
    /// allocator.
    ///
    /// This function forwards the call to [`Heap::allocate`].
    ///
    /// # Errors
    ///
    /// Errors are returned when allocation fails, see [`Error`] for more
    /// information.
    #[inline]
    pub fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<()>, core::alloc::AllocError> {
        thread::with_lazy(|heap, fallback| {
            let options = unsafe { AllocateOptions::new(fallback) };
            heap.allocate_with(layout, false, options)
        })
    }

    /// Allocate a zeroed memory block of `layout` from the current heap.
    ///
    /// The allocation can be deallocated by any instance of this configured
    /// allocator.
    ///
    /// This function forwards the call to [`Heap::allocate_zeroed`].
    ///
    /// # Errors
    ///
    /// Errors are returned when allocation fails, see [`Error`] for more
    /// information.
    #[inline]
    pub fn allocate_zeroed(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<()>, core::alloc::AllocError> {
        thread::with_lazy(|heap, fallback| {
            let options = unsafe { AllocateOptions::new(fallback) };
            heap.allocate_with(layout, true, options)
        })
    }

    /// Retrieves the layout information of a specific allocation.
    ///
    /// The layout returned may not be the same of the layout passed to
    /// [`allocate`](Heap::allocate), but is the most fit layout of it, and can
    /// be passed to [`deallocate`](Heap::deallocate).
    ///
    /// This function forwards the call to [`Heap::layout_of`].
    ///
    /// # Safety
    ///
    /// - `ptr` must point to an owned, valid memory block of `layout`, previously
    ///   allocated by a certain instance of `Heap` alive in the scope, created
    ///   from the same arena.
    /// - The allocation size must not be 0.
    #[inline]
    pub unsafe fn layout_of(&self, ptr: core::ptr::NonNull<u8>) -> core::alloc::Layout {
        thread::with(|heap| heap.layout_of(ptr))
    }

    /// Deallocates an allocation previously allocated by an instance of this
    /// type.
    ///
    /// This function forwards the call to [`Heap::deallocate`].
    ///
    /// # Safety
    ///
    /// See [`core::alloc::Allocator::deallocate`] for more information.
    #[inline]
    pub unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        thread::with(|heap| heap.deallocate(ptr, layout))
    }
}
unsafe impl core::alloc::Allocator for Custom {
    #[inline]
    fn allocate(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        thread::with_lazy(|heap, fallback| {
            let options = unsafe { AllocateOptions::new(fallback) };
            heap.allocate_with(layout, false, options)
                .map(|t| core::ptr::NonNull::from_raw_parts(t, layout.size()))
        })
    }
    #[inline]
    fn allocate_zeroed(
        &self,
        layout: core::alloc::Layout,
    ) -> Result<core::ptr::NonNull<[u8]>, core::alloc::AllocError> {
        thread::with_lazy(|heap, fallback| {
            let options = unsafe { AllocateOptions::new(fallback) };
            heap.allocate_with(layout, true, options)
                .map(|t| core::ptr::NonNull::from_raw_parts(t, layout.size()))
        })
    }
    #[inline]
    unsafe fn deallocate(&self, ptr: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        self.deallocate(ptr, layout)
    }
}
unsafe impl core::alloc::GlobalAlloc for Custom {
    #[inline]
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        thread::with_lazy(|heap, fallback| {
            heap.allocate_with(layout, false, Heap::options().fallback(fallback))
                .map_or(core::ptr::null_mut(), |ptr| ptr.as_ptr().cast())
        })
    }
    #[inline]
    unsafe fn alloc_zeroed(&self, layout: core::alloc::Layout) -> *mut u8 {
        thread::with_lazy(|heap, fallback| {
            heap.allocate_with(layout, true, Heap::options().fallback(fallback))
                .map_or(core::ptr::null_mut(), |ptr| ptr.as_ptr().cast())
        })
    }
    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        if let Some(ptr) = core::ptr::NonNull::new(ptr) {
            self.deallocate(ptr, layout)
        }
    }
}
