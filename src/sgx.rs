/// Return the base address of the currently loaded SGX enclave binary. Vendoring
/// this lets us avoid requiring the unstable `sgx_platform` feature.
///
/// This is copied from: [std::os::fortanix_sgx::mem::image_base](https://github.com/rust-lang/rust/blob/master/library/std/src/sys/sgx/abi/mem.rs#L37)
// NOTE: Do not remove inline: will result in relocation failure.
#[cfg(all(target_vendor = "fortanix", target_env = "sgx"))]
#[inline(always)]
fn image_base() -> *mut () {
    use std::arch::asm;

    let base: *mut ();
    unsafe {
        asm!(
            // `IMAGE_BASE` is defined here:
            // [std/src/sys/sgx/abi/entry.S](https://github.com/rust-lang/rust/blob/master/library/std/src/sys/sgx/abi/entry.S#L5)
            "lea IMAGE_BASE(%rip), {}",
            lateout(reg) base,
            options(att_syntax, nostack, preserves_flags, nomem, pure),
        )
    };
    base
}

#[cfg(not(all(target_vendor = "fortanix", target_env = "sgx")))]
fn image_base() -> *mut () {
    ptr::null_mut()
}

// Do not remove inline: will result in relocation failure
#[inline(always)]
pub(crate) unsafe fn rel_ptr_mut(offset: usize) -> *mut () {
    image_base().wrapping_byte_add(offset)
}

extern "C" {
    static HEAP_BASE: usize;
    static HEAP_SIZE: usize;
}

/// Returns the base memory address of the heap
pub(crate) fn heap_base() -> *mut () {
    unsafe { rel_ptr_mut(HEAP_BASE) }
}

/// Returns the size of the heap
pub(crate) fn heap_size() -> usize {
    unsafe { HEAP_SIZE }
}
