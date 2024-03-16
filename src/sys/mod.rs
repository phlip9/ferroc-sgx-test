#[cfg(all(target_env = "sgx", target_vendor = "fortanix"))]
mod sgx;

#[cfg(not(all(target_env = "sgx", target_vendor = "fortanix")))]
mod unix;

#[inline(always)]
pub(crate) fn heap_base_ptr() -> *mut () {
    #[cfg(all(target_env = "sgx", target_vendor = "fortanix"))]
    return sgx::heap_base_ptr();

    #[cfg(not(all(target_env = "sgx", target_vendor = "fortanix")))]
    return unix::heap_base_ptr();
}

#[inline(always)]
pub(crate) fn heap_size() -> usize {
    #[cfg(all(target_env = "sgx", target_vendor = "fortanix"))]
    return sgx::heap_size();

    #[cfg(not(all(target_env = "sgx", target_vendor = "fortanix")))]
    return unix::heap_size();
}

#[inline(always)]
pub(crate) fn heap_end_ptr() -> *mut () {
    heap_base_ptr().wrapping_byte_add(heap_size())
}
