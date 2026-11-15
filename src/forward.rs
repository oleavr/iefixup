use crate::sys;

include!(concat!(env!("OUT_DIR"), "/forward.rs"));

pub enum Import {
    Name(&'static [u8]),
    Ordinal(u16),
}

pub unsafe fn init(real: sys::Handle) {
    for (index, import) in FORWARD_IMPORTS.iter().enumerate() {
        let name = match import {
            Import::Name(name) => name.as_ptr(),
            Import::Ordinal(ordinal) => *ordinal as usize as *const u8,
        };
        unsafe { FORWARD_TARGETS[index] = sys::GetProcAddress(real, name) as usize };
    }
}
