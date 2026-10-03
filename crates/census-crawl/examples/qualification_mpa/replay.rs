use super::Result;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{io::Write, path::Path};

pub(super) const DIRECTORY_URL: &str = "https://www.mpa.cc/SchoolPages/School.aspx";
pub(super) const STAFF_URL: &str =
    "https://www.mpa.cc/SchoolPages/School.aspx?SchoolID=3&tab=staff";
pub(super) const DIRECTORY_AT: &str = "2026-08-31T09:00:00Z";
pub(super) const STAFF_AT: &str = "2026-09-01T10:00:00Z";
const DIRECTORY: &[u8] = include_bytes!("../../tests/fixtures/mpa/directory.html");
const STAFF: &[u8] = include_bytes!("../../tests/fixtures/mpa/staff_bonny_eagle_high.aspx");

pub(super) fn seed(cache: &Path) -> Result<Vec<Value>> {
    [
        (DIRECTORY_URL, DIRECTORY, DIRECTORY_AT),
        (STAFF_URL, STAFF, STAFF_AT),
    ]
    .into_iter()
    .map(|(url, body, fetched_at)| {
        let key = cache_key(url)?;
        let metadata = serde_json::json!({
            "url": url, "method": "GET", "status": 200,
            "content_digest": digest(body), "bytes": body.len(),
            "fetched_at": fetched_at, "content_type": "text/html",
        });
        write_new(&cache.join(format!("{key}.body")), body)?;
        write_new(
            &cache.join(format!("{key}.meta.json")),
            &serde_json::to_vec_pretty(&metadata)?,
        )?;
        Ok(metadata)
    })
    .collect()
}

pub(super) fn digest(body: &[u8]) -> String {
    format!("{:x}", Sha256::digest(body))
}

pub(super) fn fixture_digest(url: &str) -> Result<String> {
    match url {
        DIRECTORY_URL => Ok(digest(DIRECTORY)),
        STAFF_URL => Ok(digest(STAFF)),
        _ => Err("unexpected qualification capture URL".into()),
    }
}
fn cache_key(url: &str) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(b"GET");
    hasher.update([0x1f]);
    hasher.update(url.as_bytes());
    hasher.update([0x1f]);
    let key = format!("{:x}", hasher.finalize());
    Ok(key
        .get(..32)
        .ok_or("SHA256 key is shorter than 32 characters")?
        .to_string())
}

pub(super) fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub(super) fn verify(cache: &Path, metadata: &[Value]) -> Result<()> {
    if metadata.len() != 2 {
        return Err("qualification requires exactly two public fixture captures".into());
    }
    metadata.iter().try_for_each(|expected| {
        let url = expected
            .get("url")
            .and_then(Value::as_str)
            .ok_or("missing capture URL")?;
        let key = cache_key(url)?;
        let body_path = cache.join(format!("{key}.body"));
        let meta_path = cache.join(format!("{key}.meta.json"));
        let body = read_bounded(&body_path, 1024 * 1024)?;
        let actual: Value = serde_json::from_slice(&read_bounded(&meta_path, 4096)?)?;
        let fixture = match url {
            DIRECTORY_URL => DIRECTORY,
            STAFF_URL => STAFF,
            _ => return Err("unexpected qualification capture URL".into()),
        };
        if actual != *expected || body != fixture {
            return Err(format!("replay changed fixture bytes or metadata for {url}").into());
        }
        Ok(())
    })
}

fn read_bounded(path: &Path, max_bytes: u64) -> Result<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let size = file.metadata()?.len();
    if size > max_bytes {
        return Err(format!(
            "qualification input exceeds {max_bytes} bytes: {}",
            path.display()
        )
        .into());
    }
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(usize::try_from(size)?)?;
    file.take(max_bytes.checked_add(1).ok_or("read bound overflow")?)
        .read_to_end(&mut bytes)?;
    if u64::try_from(bytes.len())? != size {
        return Err("qualification input changed during bounded read".into());
    }
    Ok(bytes)
}
