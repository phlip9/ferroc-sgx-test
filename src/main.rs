#![feature(raw_os_error_ty)]
#![feature(ptr_metadata)]
#![feature(allocator_api)]
#![feature(thread_local)]
#![feature(strict_provenance)]
// #![feature(sgx_platform)]

pub mod alloc;
mod alloc_expanded;
mod sys;

// #[global_allocator]
// static GLOBAL_SGX_ALLOC: crate::alloc::SgxHeapAlloc = crate::alloc::SgxHeapAlloc;

fn main() {
    println!("Hello, world!");
}
