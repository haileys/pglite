use std::path::Path;
use std::fmt::{self, Display};

pub fn to_str<'a>(log: &slog::Logger, path: &'a Path) -> Option<&'a str> {
    let path_str = path.to_str();

    if path_str.is_none() {
        slog::warn!(log, "invalid UTF-8 in path: {}", path.to_string_lossy());
    }

    path_str
}

pub struct RelPath<P>(pub P);

impl<P: AsRef<Path>> Display for RelPath<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let current_dir = std::env::current_dir().unwrap();
        let orig_path = self.0.as_ref();
        let rel_path = self.0.as_ref()
            .strip_prefix(&current_dir)
            .unwrap_or(orig_path);
        write!(f, "{}", rel_path.to_string_lossy())
    }
}

impl<P: AsRef<Path>> slog::Value for RelPath<P> {
    fn serialize(
        &self,
        _rec: &slog::Record,
        key: slog::Key,
        ser: &mut dyn slog::Serializer
    ) -> slog::Result {
        ser.emit_str(key, &self.to_string())
    }
}
