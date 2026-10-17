use std::io::{Read, Write};
use std::path::Path;

use serde::de::DeserializeOwned;

use crate::TestResult;

pub(crate) fn write_new(path: &Path, bytes: &[u8]) -> TestResult {
    check!(bytes.len() <= 1_048_576);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> TestResult<T> {
    let metadata = std::fs::symlink_metadata(path)?;
    check!(metadata.is_file() && metadata.len() > 0 && metadata.len() <= 1_048_576);
    let file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    check!(metadata.is_file() && metadata.len() > 0 && metadata.len() <= 1_048_576);
    let mut bytes = Vec::new();
    file.take(1_048_577).read_to_end(&mut bytes)?;
    check!(bytes.len() <= 1_048_576);
    Ok(serde_json::from_slice(&bytes)?)
}

#[test]
fn fixture_transport_preserves_bytes_and_refuses_overwrite() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("plan.json");
    let bytes = br#"{"value":17}"#;
    write_new(&path, bytes)?;
    check!(eq; std::fs::read(&path)?.as_slice(), bytes.as_slice());
    check!(eq; read_json::<serde_json::Value>(&path)?, serde_json::json!({"value": 17}));
    let error = write_new(&path, b"changed").err().ok_or("overwrite accepted")?;
    check!(eq; error.downcast_ref::<std::io::Error>().ok_or("overwrite error")?.kind(), std::io::ErrorKind::AlreadyExists);
    check!(eq; std::fs::read(&path)?.as_slice(), bytes.as_slice());
    Ok(())
}

#[test]
fn fixture_transport_enforces_json_and_regular_file_bounds() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("plan.json");
    let mut bytes = vec![b' '; 1_048_576];
    *bytes.first_mut().ok_or("fixture buffer")? = b'0';
    write_new(&path, &bytes)?;
    check!(eq; read_json::<u8>(&path)?, 0);
    let oversized = directory.path().join("oversized.json");
    bytes.push(b' ');
    check!(write_new(&oversized, &bytes).is_err());
    check!(!oversized.exists());
    std::fs::write(&oversized, &bytes)?;
    check!(read_json::<serde_json::Value>(&oversized).is_err());
    for (name, bytes) in [("empty", &b""[..]), ("invalid", &b"{"[..])] {
        let path = directory.path().join(name);
        write_new(&path, bytes)?;
        check!(read_json::<serde_json::Value>(&path).is_err());
    }
    check!(read_json::<serde_json::Value>(directory.path()).is_err());
    let link = directory.path().join("link.json");
    std::os::unix::fs::symlink(&path, &link)?;
    check!(read_json::<serde_json::Value>(&link).is_err());
    Ok(())
}
