use super::*;
use std::sync::{Arc, Barrier};

#[test]
fn an_in_progress_cache_writer_retains_the_response_without_retry_or_partial_replacement() {
    let root = tempfile::tempdir().expect("cache directory");
    let (body_path, meta_path) = cache_paths(root.path());
    let original = metadata(b"old");
    write_cache(&body_path, &meta_path, b"old", &original).expect("initial capture");
    let lock = archive_io::lock_cache(&meta_path).expect("other writer owns cache");
    let result = write_cache(&body_path, &meta_path, b"new", &metadata(b"new"));
    assert!(
        matches!(result, Err(FetchError::Cache { source, .. }) if source.kind() == ErrorKind::WouldBlock)
    );
    assert_capture(root.path(), b"old", &original);
    assert_capture(root.path(), b"new", &metadata(b"new"));
    assert_eq!(std::fs::read(&body_path).expect("cache unchanged"), b"old");
    drop(lock);
}

#[test]
fn simultaneous_identical_and_different_bodies_keep_each_metadata_binding_exact() {
    let root = tempfile::tempdir().expect("cache directory");
    let barrier = Arc::new(Barrier::new(4));
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|index| {
                let barrier = Arc::clone(&barrier);
                let root = root.path();
                scope.spawn(move || {
                    let body: &[u8] = if index < 2 {
                        b"shared raw"
                    } else {
                        b"distinct raw"
                    };
                    let mut meta = metadata(body);
                    meta.url = if index < 2 {
                        "https://example.test/shared-source".to_owned()
                    } else {
                        format!("https://example.test/source/{index}")
                    };
                    let body_path = root.join(format!("source-{index}.body"));
                    let meta_path = root.join(format!("source-{index}.meta.json"));
                    barrier.wait();
                    write_cache(&body_path, &meta_path, body, &meta)
                        .expect("concurrent publication");
                    assert_capture(root, body, &meta);
                    let (served, raw) = read_cache(&body_path, &meta_path)
                        .expect("cache read")
                        .expect("hit");
                    assert_eq!(served.url, meta.url);
                    assert_eq!(raw, body);
                    meta
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("publisher thread"))
            .collect::<Vec<_>>()
    });
    let mut expected_bodies = results
        .iter()
        .map(|meta| {
            archive_body(root.path(), &meta.content_digest)
                .file_name()
                .expect("name")
                .to_owned()
        })
        .collect::<Vec<_>>();
    expected_bodies.sort();
    expected_bodies.dedup();
    assert_eq!(
        file_names(&root.path().join("archive/bodies")),
        expected_bodies
    );
    let mut shared_meta = metadata(b"shared raw");
    shared_meta.url = "https://example.test/shared-source".to_owned();
    let shared = archive_meta(root.path(), &shared_meta);
    assert_eq!(
        file_names(shared.parent().expect("captures")),
        vec![shared.file_name().expect("name").to_owned()]
    );
}

#[test]
fn same_cache_races_never_bind_one_writers_body_to_another_writers_metadata() {
    let root = tempfile::tempdir().expect("cache directory");
    let barrier = Arc::new(Barrier::new(4));
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|index| {
                let barrier = Arc::clone(&barrier);
                let root = root.path();
                scope.spawn(move || {
                    let body = format!("raw body {index}");
                    let mut meta = metadata(body.as_bytes());
                    meta.fetched_at = format!("2026-09-27T12:34:0{index}Z");
                    let (body_path, meta_path) = cache_paths(root);
                    barrier.wait();
                    let result = write_cache(&body_path, &meta_path, body.as_bytes(), &meta);
                    (body, meta, result)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("publisher thread"))
            .collect::<Vec<_>>()
    });
    let (body_path, meta_path) = cache_paths(root.path());
    let (served, raw) = read_cache(&body_path, &meta_path)
        .expect("cache read")
        .expect("coherent hit");
    assert_eq!(content_digest(&raw), served.content_digest);
    let winner = results
        .iter()
        .find(|(_, meta, _)| meta.content_digest == served.content_digest)
        .expect("winning capture");
    assert_eq!(served.fetched_at, winner.1.fetched_at);
    assert_eq!(raw, winner.0.as_bytes());
    for (body, meta, result) in results {
        match result {
            Ok(()) => assert_capture(root.path(), body.as_bytes(), &meta),
            Err(FetchError::Cache { source, .. }) => {
                assert_eq!(source.kind(), ErrorKind::WouldBlock);
                assert_capture(root.path(), body.as_bytes(), &meta);
            }
            Err(error) => panic!("unexpected publication error: {error}"),
        }
    }
}
