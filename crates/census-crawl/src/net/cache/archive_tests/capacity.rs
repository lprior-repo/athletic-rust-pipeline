use super::super::archive::admission::{capacity_of, DEFAULT_ARCHIVE_BYTES};
use super::super::archive::{preserve_capture, ArchiveAdmission};
use super::*;

fn archive_dirs(root: &Path) -> TestResult<PathBuf> {
    let archive = root.join("archive");
    std::fs::create_dir_all(archive.join("bodies"))?;
    std::fs::create_dir_all(archive.join("captures"))?;
    Ok(archive)
}

fn archive_stage(root: &Path, name: &str) -> TestResult<PathBuf> {
    let stage = root.join(name);
    std::fs::create_dir(&stage)?;
    Ok(stage)
}

fn capture_of(root: &Path, body: &[u8]) -> TestResult<capture::Capture> {
    let (_, meta_path) = cache_paths(root);
    Ok(capture::Capture::from_meta(&metadata(body), &meta_path)?)
}

fn needed_bytes(body: &[u8], capture: &capture::Capture) -> u64 {
    body.len() as u64 + capture.encoded.len() as u64
}

#[test]
fn configured_capacity_of_honors_positive_integers_and_falls_back_otherwise() -> TestResult {
    check!(eq; capacity_of(Some("2048".to_string())), 2048);
    check!(eq; capacity_of(Some("  4096 \n".to_string())), 4096);
    check!(eq; capacity_of(None), DEFAULT_ARCHIVE_BYTES);
    check!(eq; capacity_of(Some("0".to_string())), DEFAULT_ARCHIVE_BYTES);
    check!(eq; capacity_of(Some("twelve".to_string())), DEFAULT_ARCHIVE_BYTES);
    check!(eq; capacity_of(Some("".to_string())), DEFAULT_ARCHIVE_BYTES);
    Ok(())
}

#[test]
fn a_capture_over_capacity_is_refused_before_any_object_is_published() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = archive_dirs(root.path())?;
    let body = b"capture body";
    let capture = capture_of(root.path(), body)?;
    let needed = needed_bytes(body, &capture);
    let mut admission = ArchiveAdmission::begin_with_capacity(root.path(), &archive, needed - 1)?;
    let stage = archive_stage(root.path(), "stage-refused")?;
    let outcome = preserve_capture(&archive, &stage, "new", body, &capture, &mut admission);
    let Err(error) = outcome else {
        return Err("a capacity refusal should leave the capture unpublished".into());
    };
    check!(
        matches!(
            &error,
            FetchError::ArchiveCapacity { used: 0, requested, capacity, .. }
                if *requested == needed && *capacity == needed - 1
        ),
        "the refusal is typed and reports the exhausted scope: {error}"
    );
    check!(!archive_body(root.path(), &capture.meta.content_digest).exists());
    check!(!archive_meta(root.path(), &capture.meta)?.exists());
    Ok(())
}

#[test]
fn a_capture_that_fits_is_published_and_settles_the_reservation() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = archive_dirs(root.path())?;
    let body = b"capture body";
    let capture = capture_of(root.path(), body)?;
    let needed = needed_bytes(body, &capture);
    let mut admission = ArchiveAdmission::begin_with_capacity(root.path(), &archive, needed)?;
    let stage = archive_stage(root.path(), "stage-fits")?;
    preserve_capture(&archive, &stage, "new", body, &capture, &mut admission)?;
    assert_capture(root.path(), body, &capture.meta)?;
    drop(admission);
    let mut replay = ArchiveAdmission::begin_with_capacity(root.path(), &archive, needed)?;
    let replay_stage = archive_stage(root.path(), "stage-replay")?;
    preserve_capture(&archive, &replay_stage, "new", body, &capture, &mut replay)?;
    Ok(())
}

#[test]
fn an_archived_capture_settles_without_a_second_reservation() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = archive_dirs(root.path())?;
    let body = b"shared raw body";
    let capture = capture_of(root.path(), body)?;
    let needed = needed_bytes(body, &capture);
    let mut first = ArchiveAdmission::begin_with_capacity(root.path(), &archive, needed)?;
    let first_stage = archive_stage(root.path(), "stage-first")?;
    preserve_capture(&archive, &first_stage, "new", body, &capture, &mut first)?;
    drop(first);
    let mut second = ArchiveAdmission::begin_with_capacity(root.path(), &archive, needed)?;
    let second_stage = archive_stage(root.path(), "stage-second")?;
    preserve_capture(&archive, &second_stage, "old", body, &capture, &mut second)?;
    check!(
        eq; file_names(&root.path().join("archive/bodies"))?.len(), 1,
        "an already-archived body is not reserved or written twice"
    );
    Ok(())
}

#[test]
fn invalid_archive_entries_refuse_capacity_accounting() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = archive_dirs(root.path())?;
    std::os::unix::fs::symlink("bodies", archive.join("linked"))?;
    let outcome = ArchiveAdmission::begin_with_capacity(root.path(), &archive, 1 << 20);
    let Err(error) = outcome else {
        return Err("accounting over a symbolic link should be refused".into());
    };
    check!(
        error.to_string().contains("symbolic link"),
        "the inventory failure names the offending entry: {error}"
    );
    Ok(())
}

#[test]
fn concurrent_publishers_share_one_capacity_budget() -> TestResult {
    let root = tempfile::tempdir()?;
    let archive = archive_dirs(root.path())?;
    let first_body = b"first raw!!";
    let second_body = b"second raw!";
    let first = capture_of(root.path(), first_body)?;
    let second = capture_of(root.path(), second_body)?;
    check!(eq; first_body.len(), second_body.len());
    check!(eq; first.encoded.len(), second.encoded.len());
    let capacity = needed_bytes(first_body, &first) * 2 - 1;
    let stages: Vec<PathBuf> = ["stage-0", "stage-1"]
        .into_iter()
        .map(|name| archive_stage(root.path(), name))
        .collect::<TestResult<_>>()?;
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = [
            (first_body.as_slice(), &first),
            (second_body.as_slice(), &second),
        ]
        .into_iter()
        .zip(stages)
        .map(|((body, capture), stage)| {
            let root = root.path();
            let archive = archive.clone();
            scope.spawn(move || {
                let mut admission =
                    ArchiveAdmission::begin_with_capacity(root, &archive, capacity)?;
                preserve_capture(&archive, &stage, "new", body, capture, &mut admission)
            })
        })
        .collect();
        handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .map_err(|_| "publisher thread panicked".to_string())
            })
            .collect::<Vec<_>>()
    });
    let mut published = 0;
    let mut refused = 0;
    for result in results {
        match result.map_err(std::io::Error::other)? {
            Ok(_) => published += 1,
            Err(FetchError::ArchiveCapacity { capacity: held, .. }) => {
                check!(eq; held, capacity);
                refused += 1;
            }
            Err(error) => return Err(error.into()),
        }
    }
    check!(eq; published, 1, "the shared budget admits one capture");
    check!(eq; refused, 1, "the second capture is refused, not oversubscribed");
    Ok(())
}
