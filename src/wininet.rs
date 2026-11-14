use crate::forward;
use crate::html::HtmlRewriter;
use crate::registry;
use crate::stream::Stream;
use crate::sys::{self, Bool, Handle};

use core::ffi::c_void;
use core::mem::transmute;

const HTTP_QUERY_CONTENT_TYPE: u32 = 1;

type QueryDataAvailableFn = unsafe extern "system" fn(Handle, *mut u32, u32, usize) -> Bool;
type ReadFileFn = unsafe extern "system" fn(Handle, *mut c_void, u32, *mut u32) -> Bool;
type CloseHandleFn = unsafe extern "system" fn(Handle) -> Bool;
type HttpQueryInfoFn =
    unsafe extern "system" fn(Handle, u32, *mut c_void, *mut u32, *mut u32) -> Bool;

struct RealWininet {
    query_data_available: QueryDataAvailableFn,
    read_file: ReadFileFn,
    close_handle: CloseHandleFn,
    http_query_info: HttpQueryInfoFn,
}

static mut REAL: Option<RealWininet> = None;

#[unsafe(no_mangle)]
pub unsafe extern "system" fn DllMain(
    _instance: Handle,
    reason: u32,
    _reserved: *mut c_void,
) -> Bool {
    if reason == sys::DLL_PROCESS_ATTACH {
        unsafe { load_real_wininet() };
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn InternetQueryDataAvailable(
    file: Handle,
    available: *mut u32,
    flags: u32,
    context: usize,
) -> Bool {
    let stream = stream_for_handle(file);
    if !stream.is_rewriting() || !stream.has_pending_output() {
        return unsafe { (real().query_data_available)(file, available, flags, context) };
    }

    unsafe { *available = stream.pending_len() as u32 };
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn InternetReadFile(
    file: Handle,
    buffer: *mut c_void,
    size: u32,
    size_read: *mut u32,
) -> Bool {
    let stream = stream_for_handle(file);
    if !stream.is_rewriting() {
        return unsafe { (real().read_file)(file, buffer, size, size_read) };
    }

    let buffer = unsafe { core::slice::from_raw_parts_mut(buffer as *mut u8, size as usize) };

    if !stream.has_pending_output() {
        let mut n = 0;
        if unsafe { (real().read_file)(file, buffer.as_mut_ptr() as *mut c_void, size, &mut n) }
            == 0
        {
            return 0;
        }
        stream.feed(&buffer[..n as usize]);
    }

    unsafe { *size_read = stream.read(buffer) as u32 };
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn InternetCloseHandle(handle: Handle) -> Bool {
    registry::remove(handle as usize);
    unsafe { (real().close_handle)(handle) }
}

unsafe fn load_real_wininet() {
    let mut path = [0u8; sys::MAX_PATH];
    let length = unsafe { sys::GetSystemDirectoryA(path.as_mut_ptr(), path.len() as u32) } as usize;
    path[length..length + 13].copy_from_slice(b"\\WININET.DLL\0");
    let module = unsafe { sys::LoadLibraryA(path.as_ptr()) };

    unsafe {
        forward::init(module);
        REAL = Some(RealWininet {
            query_data_available: transmute(sys::GetProcAddress(
                module,
                c"InternetQueryDataAvailable".as_ptr().cast(),
            )),
            read_file: transmute(sys::GetProcAddress(
                module,
                c"InternetReadFile".as_ptr().cast(),
            )),
            close_handle: transmute(sys::GetProcAddress(
                module,
                c"InternetCloseHandle".as_ptr().cast(),
            )),
            http_query_info: transmute(sys::GetProcAddress(
                module,
                c"HttpQueryInfoA".as_ptr().cast(),
            )),
        });
    }
}

fn real() -> &'static RealWininet {
    unsafe { (*(&raw const REAL)).as_ref().unwrap_unchecked() }
}

fn stream_for_handle(handle: Handle) -> &'static mut Stream {
    match registry::find(handle as usize) {
        Some(stream) => stream,
        None => {
            let rewriter = is_html_response(handle).then(HtmlRewriter::default);
            registry::add(handle as usize, Stream::new(rewriter))
        }
    }
}

fn is_html_response(handle: Handle) -> bool {
    let mut content_type = [0u8; 128];
    let mut size = content_type.len() as u32;

    let error = unsafe { sys::GetLastError() };
    let found = unsafe {
        (real().http_query_info)(
            handle,
            HTTP_QUERY_CONTENT_TYPE,
            content_type.as_mut_ptr() as *mut c_void,
            &mut size,
            core::ptr::null_mut(),
        )
    } != 0;
    unsafe { sys::SetLastError(error) };

    let is_html = found
        && content_type[..size as usize]
            .to_ascii_lowercase()
            .starts_with(b"text/html");
    log!(
        "stream {:08x}: {}",
        handle as usize,
        if is_html {
            "rewriting HTML"
        } else {
            "passing through"
        }
    );

    is_html
}
