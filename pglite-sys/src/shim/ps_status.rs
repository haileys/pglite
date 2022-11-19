use std::os::raw::{c_int, c_char};

#[no_mangle]
pub extern "C" fn save_ps_display_args(
    _argc: c_int,
    argv: *mut *mut c_char,
) -> *mut *mut c_char {
    argv
}

#[no_mangle]
pub extern "C" fn init_ps_display(
    _fixed_part: *const c_char,
) {}

#[no_mangle]
pub extern "C" fn set_ps_display(
    _activity: *const c_char,
) {}

#[no_mangle]
pub extern "C" fn get_ps_display(
    displen: &mut c_int,
) -> *const c_char {
    *displen = 0;
    std::ptr::null()
}
