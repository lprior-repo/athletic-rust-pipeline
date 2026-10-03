use super::*;
use std::io::Write;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn metadata(body: &[u8]) -> CacheMeta {
    CacheMeta {
        url: "https://example.test/capture".to_owned(),
        response_url: None,
        method: "GET".to_owned(),
        status: 200,
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-08-26T00:00:00Z".to_owned(),
        etag: None,
        last_modified: None,
        content_type: None,
    }
}

#[test]
fn exact_limit_metadata_is_admitted_and_oversized_metadata_recovers_with_evidence() -> TestResult {
    let dir = tempfile::tempdir()?;
    let body_path = dir.path().join("capture.body");
    let meta_path = dir.path().join("capture.meta.json");
    let meta = metadata(b"captured body");
    write_cache(&body_path, &meta_path, b"captured body", &meta)?;
    let mut encoded = serde_json::to_vec(&meta)?;
    check!(encoded.len() < MAX_META_BYTES);
    encoded.resize(MAX_META_BYTES, b' ');
    std::fs::write(&meta_path, &encoded)?;
    let (_, body) = read_cache(&body_path, &meta_path)?.ok_or("hit")?;
    check!(eq; body, b"captured body");
    std::fs::OpenOptions::new()
        .append(true)
        .open(&meta_path)?
        .write_all(b" ")?;
    check!(read_cache(&body_path, &meta_path)?.is_none());
    encoded.push(b' ');
    let fresh = metadata(b"fresh body");
    write_cache(&body_path, &meta_path, b"fresh body", &fresh)?;
    let (_, served) = read_cache(&body_path, &meta_path)?.ok_or("hit")?;
    check!(eq; served, b"fresh body");
    let quarantine = dir.path().join("quarantine");
    let bundles: Vec<_> = std::fs::read_dir(quarantine)?
        .map(|entry| Ok(entry?.path()))
        .collect::<TestResult<_>>()?;
    check!(eq; bundles.len(), 1);
    let bundle = bundles.first().ok_or("bundle")?;
    check!(eq;
        std::fs::read(bundle.join("meta.json"))?,
        encoded
    );
    check!(eq;
        std::fs::read(bundle.join("body"))?,
        b"captured body"
    );
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(bundle.join("reason.json"))?)?;
    check!(eq; record["reason"], "metadata_limit_or_changed");
    Ok(())
}

#[test]
fn body_accepts_empty_and_exact_limit_but_refuses_a_false_small_declaration() -> TestResult {
    let dir = tempfile::tempdir()?;
    let body_path = dir.path().join("capture.body");
    let meta_path = dir.path().join("capture.meta.json");
    for size in [0, MAX_BODY_BYTES] {
        let body = vec![b'x'; size];
        let meta = metadata(&body);
        write_cache(&body_path, &meta_path, &body, &meta)?;
        let (_, cached) = read_cache(&body_path, &meta_path)?.ok_or("hit")?;
        check!(eq; cached, body);
    }
    let meta = metadata(b"x");
    std::fs::write(&meta_path, serde_json::to_vec(&meta)?)?;
    std::fs::remove_file(&body_path)?;
    let size = u64::try_from(MAX_BODY_BYTES)? + 1;
    std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&body_path)?
        .set_len(size)?;
    check!(read_cache(&body_path, &meta_path)?.is_none());
    Ok(())
}

struct ChangingFile {
    file: File,
    next_length: Option<u64>,
    bytes_read: usize,
}

impl Read for ChangingFile {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if let Some(length) = self.next_length.take() {
            self.file.set_len(length)?;
        }
        let read = self.file.read(buffer)?;
        self.bytes_read += read;
        Ok(read)
    }
}

#[test]
fn a_file_changing_after_preflight_is_refused_with_at_most_one_probe_byte() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("changing.body");
    let grown_length = u64::try_from(MAX_BODY_BYTES)? + 1;
    for next_length in [3, grown_length] {
        std::fs::write(&path, b"body")?;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)?;
        let expected = usize::try_from(file.metadata()?.len())?;
        let mut reader = ChangingFile {
            file,
            next_length: Some(next_length),
            bytes_read: 0,
        };
        check!(read_snapshot(&mut reader, expected)?.is_none());
        check!(reader.bytes_read <= expected + 1);
    }
    Ok(())
}

#[test]
fn an_eligible_body_io_failure_remains_a_typed_cache_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let body_path = dir.path().join("body-is-a-directory");
    let meta_path = dir.path().join("capture.meta.json");
    std::fs::create_dir(&body_path)?;
    let mut meta = metadata(b"");
    meta.bytes = usize::try_from(std::fs::metadata(&body_path)?.len())?;
    std::fs::write(&meta_path, serde_json::to_vec(&meta)?)?;
    check!(matches!(
        read_cache(&body_path, &meta_path),
        Err(FetchError::Cache { .. })
    ));
    for status in [404, 500] {
        meta.status = status;
        std::fs::write(&meta_path, serde_json::to_vec(&meta)?)?;
        check!(read_cache(&body_path, &meta_path)?.is_none());
    }
    Ok(())
}

#[test]
fn response_provenance_over_metadata_limit_cannot_publish_cache_or_archive(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let body_path = root.path().join("source.body");
    let meta_path = root.path().join("source.meta.json");
    let mut meta = metadata(b"body");
    meta.response_url = Some("x".repeat(MAX_META_BYTES.saturating_add(1)));
    check!(matches!(
        write_cache(&body_path, &meta_path, b"body", &meta),
        Err(FetchError::Cache { .. })
    ));
    check!(!body_path.exists());
    check!(!meta_path.exists());
    check!(!root.path().join("archive/bodies").exists());
    Ok(())
}

#[test]
fn historical_capture_replay_keeps_absent_final_url_and_original_serialized_bytes(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let body_path = root.path().join("source.body");
    let meta_path = root.path().join("source.meta.json");
    let raw = b"historical evidence";
    let encoded = format!(
        "{{\"url\":\"https://example.test/source\",\"method\":\"GET\",\"status\":200,\"content_digest\":\"{}\",\"bytes\":{},\"fetched_at\":\"2026-09-27T00:00:00Z\"}}",
        content_digest(raw), raw.len()
    );
    std::fs::write(&body_path, raw)?;
    std::fs::write(&meta_path, encoded.as_bytes())?;
    let (meta, body) = read_cache(&body_path, &meta_path)?.ok_or("legacy capture lost")?;
    check!(eq; meta.response_url, None);
    check!(eq; body, raw);
    check!(eq;
        replay_cache(&body_path, &meta_path, &meta)?,
        Some(raw.to_vec())
    );
    check!(eq; std::fs::read(&meta_path)?, encoded.as_bytes());
    let archived = capture::Capture::from_meta(&meta, &meta_path)?;
    let archived_path = root
        .path()
        .join("archive/captures")
        .join(&meta.content_digest)
        .join(format!("{}.meta.json", content_digest(&archived.encoded)));
    let preserved = std::fs::read(archived_path)?;
    check!(eq; preserved, archived.encoded);
    let record: serde_json::Value = serde_json::from_slice(&preserved)?;
    check!(record.get("response_url").is_none());
    Ok(())
}
