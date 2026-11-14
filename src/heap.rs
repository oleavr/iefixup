use crate::sys;

use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_void;

struct ProcessHeap;

#[global_allocator]
static ALLOCATOR: ProcessHeap = ProcessHeap;

unsafe impl GlobalAlloc for ProcessHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { sys::HeapAlloc(sys::GetProcessHeap(), 0, layout.size()) as *mut u8 }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe { sys::HeapFree(sys::GetProcessHeap(), 0, ptr as *mut c_void) };
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe {
            sys::HeapAlloc(sys::GetProcessHeap(), sys::HEAP_ZERO_MEMORY, layout.size()) as *mut u8
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, _layout: Layout, new_size: usize) -> *mut u8 {
        unsafe {
            sys::HeapReAlloc(sys::GetProcessHeap(), 0, ptr as *mut c_void, new_size) as *mut u8
        }
    }
}
