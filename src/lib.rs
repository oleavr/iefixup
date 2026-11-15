#![cfg_attr(windows, no_std)]

extern crate alloc;

pub mod css;
#[cfg(windows)]
mod forward;
pub mod headers;
#[cfg(windows)]
mod heap;
pub mod html;
#[cfg(windows)]
#[macro_use]
mod log;
#[cfg(windows)]
mod registry;
pub mod rewriter;
mod selector;
pub mod stream;
mod style;
#[cfg(windows)]
mod sys;
mod utf8;
mod value;
#[cfg(windows)]
mod wininet;

#[cfg(windows)]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::arch::asm!("ud2", options(noreturn)) }
}
