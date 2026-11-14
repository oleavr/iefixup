use core::ffi::c_void;

pub type Handle = *mut c_void;
pub type Bool = i32;

pub const DLL_PROCESS_ATTACH: u32 = 1;
pub const HEAP_ZERO_MEMORY: u32 = 8;
pub const MAX_PATH: usize = 260;

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn GetProcessHeap() -> Handle;
    pub fn HeapAlloc(heap: Handle, flags: u32, size: usize) -> *mut c_void;
    pub fn HeapReAlloc(heap: Handle, flags: u32, mem: *mut c_void, size: usize) -> *mut c_void;
    pub fn HeapFree(heap: Handle, flags: u32, mem: *mut c_void) -> Bool;
    pub fn GetSystemDirectoryA(buffer: *mut u8, size: u32) -> u32;
    pub fn LoadLibraryA(name: *const u8) -> Handle;
    pub fn GetProcAddress(module: Handle, name: *const u8) -> *const c_void;
    pub fn GetLastError() -> u32;
    pub fn SetLastError(error: u32);
    pub fn GetCurrentThreadId() -> u32;
    pub fn Sleep(milliseconds: u32);
}
