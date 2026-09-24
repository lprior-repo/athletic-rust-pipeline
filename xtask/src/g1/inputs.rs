//! Reading the retained-input directories.
//!
//! A dump of any of the three input kinds is a directory of `<name>.body` files, and the name is the
//! key the report joins on (a body's name is its sha256, an evidence or parsed record's name is
//! whatever the dump called it).

use std::fs;
use std::path::PathBuf;

/// Every `<name>.body` file under each directory, as `(stem, path)` in path order.
///
/// A directory that does not exist contributes nothing: the caller's summary line prints `0` for it.
pub(crate) fn read_dir(pattern_dirs: &[String]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for directory in pattern_dirs {
        let dir_path = PathBuf::from(directory);
        if !dir_path.is_dir() {
            continue;
        }
        let mut entries: Vec<_> = match fs::read_dir(&dir_path) {
            Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
            Err(_) => Vec::new(),
        };
        entries.sort_by_key(|e| e.path());
        for entry in entries {
            let path = entry.path();
            if path.extension().map(|e| e == "body").unwrap_or(false) {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                out.push((stem.to_string(), path.to_str().unwrap_or("").to_string()));
            }
        }
    }
    out
}
