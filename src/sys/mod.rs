#[cfg(all(target_env = "sgx", target_vendor = "fortanix"))]
mod sgx;

#[cfg(not(all(target_env = "sgx", target_vendor = "fortanix")))]
mod unix;

#[inline(always)]
pub(crate) fn heap_base() -> *mut () {
    #[cfg(all(target_env = "sgx", target_vendor = "fortanix"))]
    return sgx::heap_base();

    #[cfg(not(all(target_env = "sgx", target_vendor = "fortanix")))]
    return unix::heap_base();
}

#[inline(always)]
pub(crate) fn heap_size() -> usize {
    #[cfg(all(target_env = "sgx", target_vendor = "fortanix"))]
    return sgx::heap_size();

    #[cfg(not(all(target_env = "sgx", target_vendor = "fortanix")))]
    return unix::heap_size();
}

#[inline(always)]
pub(crate) fn heap_end() -> *mut () {
    heap_base().wrapping_byte_add(heap_size())
}
