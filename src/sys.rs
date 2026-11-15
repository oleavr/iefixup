use core::ffi::c_void;

pub type Handle = *mut c_void;
pub type Bool = i32;

pub const DLL_PROCESS_ATTACH: u32 = 1;
pub const HEAP_ZERO_MEMORY: u32 = 8;
pub const MAX_PATH: usize = 260;
pub const SM_CXSCREEN: i32 = 0;
pub const SM_CXVSCROLL: i32 = 2;
pub const SM_CXEDGE: i32 = 45;
pub const GENERIC_WRITE: u32 = 0x40000000;
pub const FILE_SHARE_READ_WRITE: u32 = 3;
pub const CREATE_ALWAYS: u32 = 2;
pub const INVALID_HANDLE_VALUE: Handle = usize::MAX as Handle;

#[repr(C)]
pub struct InternetBuffersA {
    pub struct_size: u32,
    pub next: *mut InternetBuffersA,
    pub header: *const u8,
    pub headers_length: u32,
    pub headers_total: u32,
    pub buffer: *mut c_void,
    pub buffer_length: u32,
    pub buffer_total: u32,
    pub offset_low: u32,
    pub offset_high: u32,
}

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
    pub fn CreateFileA(
        name: *const u8,
        access: u32,
        share_mode: u32,
        security: *mut c_void,
        disposition: u32,
        flags: u32,
        template: Handle,
    ) -> Handle;
    pub fn WriteFile(
        file: Handle,
        buffer: *const c_void,
        size: u32,
        written: *mut u32,
        overlapped: *mut c_void,
    ) -> Bool;
    pub fn CloseHandle(handle: Handle) -> Bool;
}

#[link(name = "user32")]
unsafe extern "system" {
    pub fn GetSystemMetrics(index: i32) -> i32;
}
