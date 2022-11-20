use std::collections::HashMap;
use std::sync::Mutex;
use std::os::raw::{c_int, c_char, c_uchar};
use std::ffi::CStr;

#[no_mangle]
unsafe extern "C" fn pglite_log(
    edata: &crate::ErrorData,
    backend_type: *const c_char,
) {
    let level = level(edata.elevel)
        // default to error log level if unknown log level:
        .unwrap_or(log::Level::Error);

    let backend = CStr::from_ptr(backend_type).to_string_lossy();

    let message = opt_cstr(edata.message);
    let filename = opt_cstr_static(edata.filename);
    let funcname = opt_cstr_static(edata.funcname);

    let lineno = match u32::try_from(edata.lineno).ok() {
        None | Some(0) => None,
        Some(line) => Some(line),
    };

    let module_path = funcname
        .map(mod_name_for_func_name)
        .unwrap_or("pglite_sys");

    let logger = log::logger();

    logger.log(&log::Record::builder()
        .level(level)
        .args(format_args!("{}", message.unwrap_or_default()))
        .target(&backend)
        // we need to use the static variants for compatibility with slog:
        .module_path_static(Some(module_path))
        .file_static(filename)
        .line(lineno)
        .build());

    logger.flush();
}

fn mod_name_for_func_name(func_name: &'static str) -> &'static str {
    lazy_static::lazy_static! {
        static ref MAP: Mutex<HashMap<&'static str, &'static str>>
            = Mutex::new(HashMap::new());
    }

    let mut map = match MAP.lock() {
        Ok(map) => map,
        // we don't care if mutex is poisoned:
        Err(poison) => poison.into_inner(),
    };

    if let Some(mod_name) = map.get(func_name) {
        return mod_name;
    }

    let mod_name = format!("pglite_sys::{}", func_name);

    // leak it
    let mod_name = Box::leak(Box::new(mod_name));

    map.insert(func_name, mod_name);
    mod_name
}

unsafe fn opt_cstr_static(ptr: *const c_char) -> Option<&'static str> {
    if ptr == std::ptr::null() {
        None
    } else {
        CStr::from_ptr(ptr).to_str().ok()
    }
}

unsafe fn opt_cstr(ptr: *const c_char) -> Option<String> {
    if ptr == std::ptr::null() {
        None
    } else {
        Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
    }
}

#[no_mangle]
unsafe extern "C" fn pglite_log_raw_(
    msg: *const c_char,
    len: usize,
    filename: *const c_char,
    lineno: c_int,
    funcname: *const c_char,
) {
    let msg = std::slice::from_raw_parts(msg as *const c_uchar, len);
    let msg = String::from_utf8_lossy(msg);

    let filename = opt_cstr_static(filename);
    let funcname = opt_cstr_static(funcname);
    let lineno = u32::try_from(lineno).ok();

    let mod_path = funcname.map(mod_name_for_func_name);

    let logger = log::logger();

    logger.log(&log::Record::builder()
        .level(log::Level::Error)
        .args(format_args!("{}", msg))
        .module_path_static(mod_path)
        .file_static(filename)
        .line(lineno)
        .build());

    logger.flush();
}

fn level(severity: c_int) -> Option<log::Level> {
    match u32::try_from(severity).ok()? {
        | crate::DEBUG1
        | crate::DEBUG2
        | crate::DEBUG3
        | crate::DEBUG4
        | crate::DEBUG5 => Some(log::Level::Trace),
        | crate::LOG
        | crate::LOG_SERVER_ONLY => Some(log::Level::Debug),
        | crate::INFO
        | crate::NOTICE => Some(log::Level::Info),
        | crate::WARNING
        | crate::WARNING_CLIENT_ONLY => Some(log::Level::Warn),
        | crate::ERROR
        | crate::FATAL
        | crate::PANIC => Some(log::Level::Error),
        _ => None,
    }
}
