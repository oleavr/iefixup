use crate::sys;

include!(concat!(env!("OUT_DIR"), "/forward.rs"));

pub unsafe fn init(real: sys::Handle) {
    for (index, name) in FORWARD_NAMES.iter().enumerate() {
        unsafe { FORWARD_TARGETS[index] = sys::GetProcAddress(real, name.as_ptr()) as usize };
    }
}
