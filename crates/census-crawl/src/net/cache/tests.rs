use super::*;
use std::io::{ErrorKind, Write};

fn metadata(body: &[u8]) -> CacheMeta {
    CacheMeta {
        url: "https://example.test/capture".to_owned(),
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
fn metadata_accepts_the_exact_encoded_limit_and_refuses_one_more_byte() {
    let dir = tempfile::tempdir().expect("cache directory");
    let body_path = dir.path().join("capture.body");
    let meta_path = dir.path().join("capture.meta.json");
    let meta = metadata(b"captured body");
    write_cache(&body_path, &meta_path, b"captured body", &meta).expect("cache seed");
    let mut encoded = serde_json::to_vec(&meta).expect("metadata encoding");
    assert!(encoded.len() < MAX_META_BYTES);
    encoded.resize(MAX_META_BYTES, b' ');
    std::fs::write(&meta_path, &encoded).expect("exact-limit metadata");
    let (_, body) = read_cache(&body_path, &meta_path).expect("cache read").expect("hit");
    assert_eq!(body, b"captured body");
    std::fs::OpenOptions::new()
        .append(true)
        .open(&meta_path)
        .expect("metadata file")
        .write_all(b" ")
        .expect("one excess byte");
    assert!(matches!(
        read_cache(&body_path, &meta_path),
        Err(FetchError::Cache { source, .. }) if source.kind() == ErrorKind::InvalidData
    ));
}

#[test]
fn body_accepts_empty_and_exact_limit_but_refuses_a_false_small_declaration() {
    let dir = tempfile::tempdir().expect("cache directory");
    let body_path = dir.path().join("capture.body");
    let meta_path = dir.path().join("capture.meta.json");
    for size in [0, MAX_BODY_BYTES] {
        let body = vec![b'x'; size];
        let meta = metadata(&body);
        write_cache(&body_path, &meta_path, &body, &meta).expect("cache seed");
        let (_, cached) = read_cache(&body_path, &meta_path).expect("cache read").expect("hit");
        assert_eq!(cached, body);
    }
    let meta = metadata(b"x");
    std::fs::write(&meta_path, serde_json::to_vec(&meta).expect("metadata encoding"))
        .expect("false declaration");
    let size = u64::try_from(MAX_BODY_BYTES).expect("body bound") + 1;
    std::fs::OpenOptions::new()
        .write(true)
        .open(&body_path)
        .expect("body file")
        .set_len(size)
        .expect("oversized sparse body");
    assert!(read_cache(&body_path, &meta_path).expect("bounded read").is_none());
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
fn a_file_changing_after_preflight_is_refused_with_at_most_one_probe_byte() {
    let dir = tempfile::tempdir().expect("cache directory");
    let path = dir.path().join("changing.body");
    let grown_length = u64::try_from(MAX_BODY_BYTES).expect("body bound") + 1;
    for next_length in [3, grown_length] {
        std::fs::write(&path, b"body").expect("initial file");
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("same open handle");
        let expected = usize::try_from(file.metadata().expect("preflight").len()).expect("length");
        let mut reader = ChangingFile { file, next_length: Some(next_length), bytes_read: 0 };
        assert!(read_snapshot(&mut reader, expected).expect("bounded snapshot").is_none());
        assert!(reader.bytes_read <= expected + 1);
    }
}

#[test]
fn an_eligible_body_io_failure_remains_a_typed_cache_error() {
    let dir = tempfile::tempdir().expect("cache directory");
    let body_path = dir.path().join("body-is-a-directory");
    let meta_path = dir.path().join("capture.meta.json");
    std::fs::create_dir(&body_path).expect("unreadable body");
    let mut meta = metadata(b"");
    meta.bytes = usize::try_from(std::fs::metadata(&body_path).expect("directory metadata").len())
        .expect("directory length");
    std::fs::write(&meta_path, serde_json::to_vec(&meta).expect("metadata encoding"))
        .expect("eligible metadata");
    assert!(matches!(read_cache(&body_path, &meta_path), Err(FetchError::Cache { .. })));
    for status in [404, 500] {
        meta.status = status;
        std::fs::write(&meta_path, serde_json::to_vec(&meta).expect("metadata encoding"))
            .expect("ineligible metadata");
        assert!(read_cache(&body_path, &meta_path).expect("body must not be read").is_none());
    }
}
