// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg_attr(stage0, feature(custom_attribute))]
#![crate_name = "alloc_jemalloc"]
#![crate_type = "rlib"]
#![cfg_attr(stage0, staged_api)]
#![no_std]
#![cfg_attr(not(stage0), allocator)]
#![cfg_attr(stage0, allow(improper_ctypes))]
#![unstable(feature = "alloc_jemalloc",
            reason = "this library is unlikely to be stabilized in its current \
                      form or name",
            issue = "27783")]
#![feature(allocator)]
#![feature(libc)]
#![feature(no_std)]
#![feature(staged_api)]

extern crate libc;

use libc::{c_int, c_void, size_t};

// Linkage directives to pull in jemalloc and its dependencies.
//
// On some platforms we need to be sure to link in `pthread` which jemalloc
// depends on, and specifically on android we need to also link to libgcc.
// Currently jemalloc is compiled with gcc which will generate calls to
// intrinsics that are libgcc specific (e.g. those intrinsics aren't present in
// libcompiler-rt), so link that in to get that support.
#[link(name = "jemalloc", kind = "static")]
#[cfg_attr(target_os = "android", link(name = "gcc"))]
#[cfg_attr(all(not(windows),
               not(target_os = "android"),
               not(target_env = "musl")),
           link(name = "pthread"))]
extern "C" {
    fn je_mallocx(size: size_t, flags: c_int) -> *mut c_void;
    fn je_rallocx(ptr: *mut c_void, size: size_t, flags: c_int) -> *mut c_void;
    fn je_xallocx(ptr: *mut c_void, size: size_t, extra: size_t, flags: c_int) -> size_t;
    fn je_sdallocx(ptr: *mut c_void, size: size_t, flags: c_int);
    fn je_nallocx(size: size_t, flags: c_int) -> size_t;
}

// The minimum alignment guaranteed by the architecture. This value is used to
// add fast paths for low alignment values. In practice, the alignment is a
// constant at the call site and the branch will be optimized out.
#[cfg(all(any(target_arch = "arm",
              target_arch = "mips",
              target_arch = "mipsel",
              target_arch = "powerpc")))]
const MIN_ALIGN: usize = 8;
#[cfg(all(any(target_arch = "x86",
              target_arch = "x86_64",
              target_arch = "aarch64")))]
const MIN_ALIGN: usize = 16;

// MALLOCX_ALIGN(a) macro
fn mallocx_align(a: usize) -> c_int {
    a.trailing_zeros() as c_int
}

fn align_to_flags(align: usize) -> c_int {
    if align <= MIN_ALIGN {
        0
    } else {
        mallocx_align(align)
    }
}

#[no_mangle]
pub extern "C" fn __rust_allocate(size: usize, align: usize) -> *mut u8 {
    let flags = align_to_flags(align);
    unsafe { je_mallocx(size as size_t, flags) as *mut u8 }
}

#[no_mangle]
pub extern "C" fn __rust_reallocate(ptr: *mut u8,
                                    _old_size: usize,
                                    size: usize,
                                    align: usize)
                                    -> *mut u8 {
    let flags = align_to_flags(align);
    unsafe { je_rallocx(ptr as *mut c_void, size as size_t, flags) as *mut u8 }
}

#[no_mangle]
pub extern "C" fn __rust_reallocate_inplace(ptr: *mut u8,
                                            _old_size: usize,
                                            size: usize,
                                            align: usize)
                                            -> usize {
    let flags = align_to_flags(align);
    unsafe { je_xallocx(ptr as *mut c_void, size as size_t, 0, flags) as usize }
}

#[no_mangle]
pub extern "C" fn __rust_deallocate(ptr: *mut u8, old_size: usize, align: usize) {
    let flags = align_to_flags(align);
    unsafe { je_sdallocx(ptr as *mut c_void, old_size as size_t, flags) }
}

#[no_mangle]
pub extern "C" fn __rust_usable_size(size: usize, align: usize) -> usize {
    let flags = align_to_flags(align);
    unsafe { je_nallocx(size as size_t, flags) as usize }
}
