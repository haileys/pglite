/// backend/main

use pglite_sys as sys;

/// Performs all necessary initialisation to ready this thread to run Postgres
pub unsafe fn thread_start() {
    tls();

    // main.c
    sys::pglite_init_dummy_spin_lock();
    sys::MemoryContextInit();
    sys::check_strxfrm_bug();
}

/// Initializes thread-local storage for this database thread
unsafe fn tls() {
    sys::pglite_tls_init_access_common_reloptions();
    sys::pglite_tls_init_access_transam_multixacct();
    sys::pglite_tls_init_access_transam_parallel();
    sys::pglite_tls_init_access_transam_xact();
    sys::pglite_tls_init_access_transam_xloginsert();
    sys::pglite_tls_init_lib_rbtree();
    sys::pglite_tls_init_postmaster_autovacuum();
    sys::pglite_tls_init_postmaster_postmaster();
    sys::pglite_tls_init_replication_logical_worker();
    sys::pglite_tls_init_storage_ipc_dsm();
    sys::pglite_tls_init_storage_lmgr_lock();
    sys::pglite_tls_init_utils_activity_pgstat();
    sys::pglite_tls_init_utils_activity_wait_event();
    sys::pglite_tls_init_utils_cache_plancache();
    sys::pglite_tls_init_utils_misc_guc();
}

