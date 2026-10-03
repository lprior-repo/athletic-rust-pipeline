use std::fs;
use std::path::PathBuf;

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
            if path
                .extension()
                .is_some_and(|extension| extension == "body")
            {
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map_or("", core::convert::identity);
                out.push((
                    stem.to_string(),
                    path.to_str()
                        .map_or("", core::convert::identity)
                        .to_string(),
                ));
            }
        }
    }
    out
}
