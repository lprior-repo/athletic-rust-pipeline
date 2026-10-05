use super::FetchError;
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions, TryLockError};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub(crate) struct OriginLocks {
    root: Option<PathBuf>,
    held: Mutex<BTreeMap<String, File>>,
}

impl OriginLocks {
    pub(crate) fn disabled() -> Self {
        Self {
            root: None,
            held: Mutex::new(BTreeMap::new()),
        }
    }

    pub(crate) fn rooted(root: PathBuf) -> Self {
        Self {
            root: Some(root),
            held: Mutex::new(BTreeMap::new()),
        }
    }

    pub(crate) fn ensure(&self, origin: &str) -> Result<(), FetchError> {
        let origin = origin.trim().to_ascii_lowercase();
        if origin.is_empty() {
            return Ok(());
        }
        let Some(root) = self.root.as_ref() else {
            return Ok(());
        };
        let mut held = match self.held.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if held.contains_key(&origin) {
            return Ok(());
        }
        std::fs::create_dir_all(root).map_err(|source| FetchError::OriginLockIo {
            path: root.clone(),
            source,
        })?;
        let path = root.join(origin_lock_file_name(&origin));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| FetchError::OriginLockIo {
                path: path.clone(),
                source,
            })?;
        match file.try_lock() {
            Ok(()) => {
                write_holder(&file, &origin, &path)?;
                held.insert(origin, file);
                Ok(())
            }
            Err(TryLockError::WouldBlock) => match holder_record(&path) {
                Some(record) if record.pid == Some(std::process::id()) => Ok(()),
                record => Err(FetchError::OriginHeld {
                    origin,
                    holder: record.map_or_else(|| "unknown".to_string(), |record| record.text),
                }),
            },
            Err(TryLockError::Error(source)) => Err(FetchError::OriginLockIo { path, source }),
        }
    }
}

pub fn origin_lock_file_name(origin: &str) -> String {
    let mut name: String = origin
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect();
    name.push_str(".lock");
    name
}

struct HolderRecord {
    pid: Option<u32>,
    text: String,
}

fn holder_record(path: &Path) -> Option<HolderRecord> {
    let mut text = String::new();
    File::open(path).ok()?.read_to_string(&mut text).ok()?;
    let text: String = text.trim().chars().take(512).collect();
    let pid = serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|value| value.get("pid").and_then(serde_json::Value::as_u64))
        .and_then(|pid| u32::try_from(pid).ok());
    Some(HolderRecord { pid, text })
}

fn write_holder(file: &File, origin: &str, path: &Path) -> Result<(), FetchError> {
    let record = serde_json::json!({
        "pid": std::process::id(),
        "origin": origin,
        "command": command_line(),
        "started_at": super::now_iso8601(),
    })
    .to_string();
    let io_error = |source| FetchError::OriginLockIo {
        path: path.to_path_buf(),
        source,
    };
    file.set_len(0).map_err(io_error)?;
    let mut writer: &File = file;
    writer.write_all(record.as_bytes()).map_err(io_error)?;
    writer.flush().map_err(io_error)
}

fn command_line() -> String {
    std::env::args().take(4).collect::<Vec<String>>().join(" ")
}

#[cfg(test)]
#[path = "origin_locks_tests.rs"]
mod tests;
