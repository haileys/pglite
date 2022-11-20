/// backend/bootstrap

use std::ffi::CStr;
use std::ptr;
use pglite_sys as sys;

pub unsafe fn main(data_dir: &CStr) {
    sys::InitStandaloneProcess();
    sys::InitializeGUCOptions();

    // this is where we would load postgresql.conf
    // guc.c SelectConfigFiles

    sys::SetDataDir(data_dir.as_ptr());
    sys::checkDataDir();

    sys::pglite_set_bootstrap_processing_mode();

    sys::InitializeMaxBackends();
    sys::CreateSharedMemoryAndSemaphores();
    sys::InitProcess();
    sys::BaseInit();
    sys::BootStrapXLOG();

    let invalid_oid: sys::Oid = 0;
    sys::InitPostgres(ptr::null(), invalid_oid, ptr::null(), invalid_oid, false, false, ptr::null_mut());

    sys::StartTransactionCommand();
    sys::boot_yyparse();
    sys::CommitTransactionCommand();

    sys::RelationMapFinishBootstrap();
}
