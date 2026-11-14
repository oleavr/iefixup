#![cfg_attr(windows, no_std)]

extern crate alloc;

#[cfg(windows)]
mod forward;
#[cfg(windows)]
mod heap;
pub mod html;
#[cfg(windows)]
#[macro_use]
mod log;
#[cfg(windows)]
mod registry;
pub mod stream;
#[cfg(windows)]
mod sys;
mod utf8;
#[cfg(windows)]
mod wininet;

#[cfg(windows)]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::arch::asm!("ud2", options(noreturn)) }
}
