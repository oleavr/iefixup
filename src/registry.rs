use crate::stream::Stream;
use crate::sys;

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

struct Entry {
    handle: usize,
    stream: Stream,
}

static LOCKED: AtomicBool = AtomicBool::new(false);
static mut ENTRIES: Vec<Box<Entry>> = Vec::new();

pub fn find(handle: usize) -> Option<&'static mut Stream> {
    with_entries(|entries| {
        let entry = entries.iter_mut().find(|entry| entry.handle == handle)?;
        Some(unsafe { &mut *(&raw mut entry.stream) })
    })
}

pub fn add(handle: usize, stream: Stream) -> &'static mut Stream {
    let mut entry = Box::new(Entry { handle, stream });
    let stream = unsafe { &mut *(&raw mut entry.stream) };
    with_entries(|entries| entries.push(entry));
    stream
}

pub fn remove(handle: usize) {
    let removed = with_entries(|entries| {
        let index = entries.iter().position(|entry| entry.handle == handle)?;
        Some(entries.swap_remove(index))
    });
    drop(removed);
}

fn with_entries<R>(f: impl FnOnce(&mut Vec<Box<Entry>>) -> R) -> R {
    while LOCKED.swap(true, Ordering::Acquire) {
        unsafe { sys::Sleep(0) };
    }
    let result = f(unsafe { &mut *(&raw mut ENTRIES) });
    LOCKED.store(false, Ordering::Release);
    result
}
