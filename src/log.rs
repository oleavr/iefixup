use crate::sys;

use core::fmt::{self, Write};

const QEMU_DEBUGCON_PORT: u16 = 0xe9;

macro_rules! log {
    ($($arg:tt)*) => {
        if cfg!(feature = "debug-log") {
            $crate::log::write(format_args!($($arg)*));
        }
    };
}

pub fn write(args: fmt::Arguments) {
    let mut console = Debugcon;
    let _ = write!(
        console,
        "[iefixup {:x}] {}\n",
        unsafe { sys::GetCurrentThreadId() },
        args
    );
}

struct Debugcon;

impl Write for Debugcon {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            unsafe { core::arch::asm!("out dx, al", in("dx") QEMU_DEBUGCON_PORT, in("al") byte) };
        }
        Ok(())
    }
}
