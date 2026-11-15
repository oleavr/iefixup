use crate::forward;
use crate::headers;
use crate::registry;
use crate::rewriter::Rewriter;
use crate::stream::Stream;
use crate::sys::{self, Bool, Handle};

use alloc::vec::Vec;
use core::ffi::{CStr, c_void};
use core::mem::transmute;

const HTTP_QUERY_CONTENT_TYPE: u32 = 1;
const HTTP_QUERY_CONTENT_ENCODING: u32 = 29;
const INTERNET_OPTION_DATAFILE_NAME: u32 = 33;
const REAL_WININET: &[u8] = b"\\WININETR.DLL\0";

type SendRequestFn = unsafe extern "system" fn(Handle, *const u8, u32, *mut c_void, u32) -> Bool;
type QueryDataAvailableFn = unsafe extern "system" fn(Handle, *mut u32, u32, usize) -> Bool;
type ReadFileFn = unsafe extern "system" fn(Handle, *mut c_void, u32, *mut u32) -> Bool;
type ReadFileExFn =
    unsafe extern "system" fn(Handle, *mut sys::InternetBuffersA, u32, usize) -> Bool;
type QueryOptionFn = unsafe extern "system" fn(Handle, u32, *mut c_void, *mut u32) -> Bool;
type CloseHandleFn = unsafe extern "system" fn(Handle) -> Bool;
type HttpQueryInfoFn =
    unsafe extern "system" fn(Handle, u32, *mut c_void, *mut u32, *mut u32) -> Bool;

struct RealWininet {
    send_request: SendRequestFn,
    query_data_available: QueryDataAvailableFn,
    read_file: ReadFileFn,
    read_file_ex: ReadFileExFn,
    query_option: QueryOptionFn,
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
pub unsafe extern "system" fn HttpSendRequestA(
    request: Handle,
    headers: *const u8,
    headers_length: u32,
    optional: *mut c_void,
    optional_length: u32,
) -> Bool {
    let send = real().send_request;
    match unsafe { headers_without_accept_encoding(headers, headers_length) } {
        Some(headers) => unsafe {
            send(
                request,
                headers.as_ptr(),
                headers.len() as u32,
                optional,
                optional_length,
            )
        },
        None => unsafe { send(request, headers, headers_length, optional, optional_length) },
    }
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
    let read = read_rewritten(file, stream, buffer, |buffer| {
        let mut n = 0;
        let ok =
            unsafe { (real().read_file)(file, buffer.as_mut_ptr() as *mut c_void, size, &mut n) }
                != 0;
        ok.then_some(n as usize)
    });

    match read {
        Some(n) => {
            unsafe { *size_read = n as u32 };
            1
        }
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn InternetReadFileExA(
    file: Handle,
    buffers: *mut sys::InternetBuffersA,
    flags: u32,
    context: usize,
) -> Bool {
    let stream = stream_for_handle(file);
    if !stream.is_rewriting() {
        return unsafe { (real().read_file_ex)(file, buffers, flags, context) };
    }

    let buffers = unsafe { &mut *buffers };
    let buffer = unsafe {
        core::slice::from_raw_parts_mut(buffers.buffer as *mut u8, buffers.buffer_length as usize)
    };
    let read = read_rewritten(file, stream, buffer, |_| {
        let ok = unsafe { (real().read_file_ex)(file, buffers, flags, context) } != 0;
        ok.then_some(buffers.buffer_length as usize)
    });

    match read {
        Some(n) => {
            buffers.buffer_length = n as u32;
            1
        }
        None => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "system" fn InternetCloseHandle(handle: Handle) -> Bool {
    registry::remove(handle as usize);
    unsafe { (real().close_handle)(handle) }
}

unsafe fn load_real_wininet() {
    let mut path = [0u8; sys::MAX_PATH];
    let length = unsafe { sys::GetSystemDirectoryA(path.as_mut_ptr(), path.len() as u32) } as usize;
    path[length..length + REAL_WININET.len()].copy_from_slice(REAL_WININET);
    let module = unsafe { sys::LoadLibraryA(path.as_ptr()) };

    unsafe {
        forward::init(module);
        REAL = Some(RealWininet {
            send_request: transmute(sys::GetProcAddress(
                module,
                c"HttpSendRequestA".as_ptr().cast(),
            )),
            query_data_available: transmute(sys::GetProcAddress(
                module,
                c"InternetQueryDataAvailable".as_ptr().cast(),
            )),
            read_file: transmute(sys::GetProcAddress(
                module,
                c"InternetReadFile".as_ptr().cast(),
            )),
            read_file_ex: transmute(sys::GetProcAddress(
                module,
                c"InternetReadFileExA".as_ptr().cast(),
            )),
            query_option: transmute(sys::GetProcAddress(
                module,
                c"InternetQueryOptionA".as_ptr().cast(),
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

unsafe fn headers_without_accept_encoding(headers: *const u8, length: u32) -> Option<Vec<u8>> {
    if headers.is_null() {
        return None;
    }
    let headers = match length {
        u32::MAX => unsafe { CStr::from_ptr(headers.cast()).to_bytes() },
        length => unsafe { core::slice::from_raw_parts(headers, length as usize) },
    };
    headers::without_accept_encoding(headers)
}

fn read_rewritten(
    file: Handle,
    stream: &mut Stream,
    buffer: &mut [u8],
    read_real: impl FnOnce(&mut [u8]) -> Option<usize>,
) -> Option<usize> {
    if !stream.has_pending_output() {
        let n = read_real(buffer)?;
        stream.feed(&buffer[..n]);
        if let Some(contents) = stream.cache_file_replacement() {
            replace_cache_file(file, contents);
        }
    }
    Some(stream.read(buffer))
}

fn replace_cache_file(handle: Handle, contents: &[u8]) {
    let mut name = [0u8; sys::MAX_PATH];
    let mut size = name.len() as u32;
    let found = unsafe {
        (real().query_option)(
            handle,
            INTERNET_OPTION_DATAFILE_NAME,
            name.as_mut_ptr() as *mut c_void,
            &mut size,
        )
    } != 0;
    if !found {
        return;
    }

    let file = unsafe {
        sys::CreateFileA(
            name.as_ptr(),
            sys::GENERIC_WRITE,
            sys::FILE_SHARE_READ_WRITE,
            core::ptr::null_mut(),
            sys::CREATE_ALWAYS,
            0,
            core::ptr::null_mut(),
        )
    };
    log!(
        "replacing cache file {:?}: {}",
        core::str::from_utf8(&name[..size as usize]),
        if file == sys::INVALID_HANDLE_VALUE {
            "failed"
        } else {
            "ok"
        }
    );
    if file == sys::INVALID_HANDLE_VALUE {
        return;
    }

    let mut written = 0;
    unsafe {
        sys::WriteFile(
            file,
            contents.as_ptr() as *const c_void,
            contents.len() as u32,
            &mut written,
            core::ptr::null_mut(),
        );
        sys::CloseHandle(file);
    }
}

fn real() -> &'static RealWininet {
    unsafe { (*(&raw const REAL)).as_ref().unwrap_unchecked() }
}

fn stream_for_handle(handle: Handle) -> &'static mut Stream {
    match registry::find(handle as usize) {
        Some(stream) => stream,
        None => registry::add(handle as usize, Stream::new(rewriter_for_response(handle))),
    }
}

fn rewriter_for_response(handle: Handle) -> Option<Rewriter> {
    let error = unsafe { sys::GetLastError() };
    let content_type = query_header(handle, HTTP_QUERY_CONTENT_TYPE);
    let encoded = query_header(handle, HTTP_QUERY_CONTENT_ENCODING).is_some();
    unsafe { sys::SetLastError(error) };

    let viewport_width = unsafe {
        sys::GetSystemMetrics(sys::SM_CXSCREEN)
            - sys::GetSystemMetrics(sys::SM_CXVSCROLL)
            - 2 * sys::GetSystemMetrics(sys::SM_CXEDGE)
    };
    let rewriter = content_type
        .filter(|_| !encoded)
        .and_then(|content_type| Rewriter::for_content_type(&content_type, viewport_width));
    log!(
        "stream {:08x}: {}",
        handle as usize,
        match rewriter {
            Some(Rewriter::Html(_)) => "rewriting HTML",
            Some(Rewriter::Css(_)) => "rewriting CSS",
            None => "passing through",
        }
    );

    rewriter
}

fn query_header(handle: Handle, info_level: u32) -> Option<Vec<u8>> {
    let mut value = [0u8; 128];
    let mut size = value.len() as u32;

    let found = unsafe {
        (real().http_query_info)(
            handle,
            info_level,
            value.as_mut_ptr() as *mut c_void,
            &mut size,
            core::ptr::null_mut(),
        )
    } != 0;

    found.then(|| value[..size as usize].to_vec())
}
