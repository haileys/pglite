/// backend/bootstrap

use std::ffi::CStr;
use pglite_sys as sys;

pub unsafe fn main(data_dir: &CStr) {
    sys::InitStandaloneProcess();
    sys::InitializeGUCOptions();

    // this is where we would load postgresql.conf
    // guc.c SelectConfigFiles

    sys::SetDataDir(data_dir.as_ptr());
}
