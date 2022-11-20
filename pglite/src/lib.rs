mod db;

use std::path::Path;
use std::ffi::CString;

use pglite_sys as sys;

pub struct Connection {

}

pub enum OpenError {
    PathNameNotUtf8,
    PathNameContainsNul,
}

impl Connection {
    pub fn open(data_dir: &Path) -> Result<Self, OpenError> {
        let data_dir = data_dir.to_str()
            .ok_or(OpenError::PathNameNotUtf8)?;

        let data_dir = CString::new(data_dir)
            .map_err(|_| OpenError::PathNameContainsNul)?;

        let thread = std::thread::spawn(move || unsafe {
            db::init::thread_start();
            db::bootstrap::main(&data_dir);
            eprintln!("pglite: survived the bootstrap!");
        });

        thread.join();

        Ok(Connection {})
    }
}
