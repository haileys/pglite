use std::error::Error;
use std::fmt;
use std::os::raw::{c_char, c_int};
use std::ffi::CStr;

#[derive(Debug, Copy, Clone)]
pub struct ExitThread {
    #[allow(unused)]
    code: c_int,
}

impl fmt::Display for ExitThread {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for ExitThread {}

#[no_mangle]
unsafe extern "C" fn pglite_exit_thread(code: c_int) {
    panic!("pglite_exit_thread: {:?}", ExitThread { code });
}

#[no_mangle]
unsafe extern "C" fn pglite_abort() {
    panic!("pglite_abort");
}

#[no_mangle]
unsafe extern "C" fn ExceptionalCondition(
    condition_name: *const c_char,
    error_type: *const c_char,
    file_name: *const c_char,
    line_number: c_int,
) {
    let condition_name = CStr::from_ptr(condition_name);
    let error_type = CStr::from_ptr(error_type);
    let file_name = CStr::from_ptr(file_name);

    panic!("pglite ExceptionalCondition: {}({:?}, File: {:?}, Line: {})",
        error_type.to_string_lossy(),
        condition_name.to_string_lossy(),
        file_name.to_string_lossy(),
        line_number,
    );
}
