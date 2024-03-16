#[inline(always)]
pub(crate) fn heap_base() -> *mut () {
    std::ptr::null_mut()
}

#[inline(always)]
pub(crate) fn heap_size() -> usize {
    0
}
