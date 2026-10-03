use super::*;
use std::sync::{Arc, Barrier};

#[test]
fn an_in_progress_cache_writer_retains_the_response_without_retry_or_partial_replacement(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let (body_path, meta_path) = cache_paths(root.path());
    let original = metadata(b"old");
    write_cache(&body_path, &meta_path, b"old", &original)?;
    let lock = archive_io::lock_cache(&meta_path)?;
    let result = write_cache(&body_path, &meta_path, b"new", &metadata(b"new"));
    check!(
        matches!(result, Err(FetchError::Cache { source, .. }) if source.kind() == ErrorKind::WouldBlock)
    );
    assert_capture(root.path(), b"old", &original)?;
    assert_capture(root.path(), b"new", &metadata(b"new"))?;
    check!(eq; std::fs::read(&body_path)?, b"old");
    drop(lock);
    Ok(())
}

#[test]
fn simultaneous_identical_and_different_bodies_keep_each_metadata_binding_exact() -> TestResult {
    let root = tempfile::tempdir()?;
    let barrier = Arc::new(Barrier::new(4));
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|index| {
                let barrier = Arc::clone(&barrier);
                let root = root.path();
                scope.spawn(move || -> TestResult<CacheMeta> {
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
                    write_cache(&body_path, &meta_path, body, &meta)?;
                    assert_capture(root, body, &meta)?;
                    let (served, raw) = read_cache(&body_path, &meta_path)?.ok_or("hit")?;
                    check!(eq; served.url, meta.url);
                    check!(eq; raw, body);
                    Ok(meta)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().map_err(|_| "publisher thread panicked")?)
            .collect::<TestResult<Vec<_>>>()
    })?;
    let mut expected_bodies = results
        .iter()
        .map(|meta| {
            Ok(archive_body(root.path(), &meta.content_digest)
                .file_name()
                .ok_or("name")?
                .to_owned())
        })
        .collect::<TestResult<Vec<_>>>()?;
    expected_bodies.sort();
    expected_bodies.dedup();
    check!(eq;
        file_names(&root.path().join("archive/bodies"))?,
        expected_bodies
    );
    let mut shared_meta = metadata(b"shared raw");
    shared_meta.url = "https://example.test/shared-source".to_owned();
    let shared = archive_meta(root.path(), &shared_meta)?;
    check!(eq;
        file_names(shared.parent().ok_or("captures")?)?,
        vec![shared.file_name().ok_or("name")?.to_owned()]
    );
    Ok(())
}

#[test]
fn same_cache_races_never_bind_one_writers_body_to_another_writers_metadata() -> TestResult {
    let root = tempfile::tempdir()?;
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
            .map(|handle| {
                handle
                    .join()
                    .map_err(|_| "publisher thread panicked".into())
            })
            .collect::<TestResult<Vec<_>>>()
    })?;
    let (body_path, meta_path) = cache_paths(root.path());
    let (served, raw) = read_cache(&body_path, &meta_path)?.ok_or("coherent hit")?;
    check!(eq; content_digest(&raw), served.content_digest);
    let winner = results
        .iter()
        .find(|(_, meta, _)| meta.content_digest == served.content_digest)
        .ok_or("winning capture")?;
    check!(eq; served.fetched_at, winner.1.fetched_at);
    check!(eq; raw, winner.0.as_bytes());
    for (body, meta, result) in results {
        match result {
            Ok(()) => assert_capture(root.path(), body.as_bytes(), &meta)?,
            Err(FetchError::Cache { source, .. }) => {
                check!(eq; source.kind(), ErrorKind::WouldBlock);
                assert_capture(root.path(), body.as_bytes(), &meta)?;
            }
            Err(error) => return Err(format!("unexpected publication error: {error}").into()),
        }
    }
    Ok(())
}
