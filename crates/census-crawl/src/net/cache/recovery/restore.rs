use super::{admission::Admission, paths, reservations, shell, stage};
use crate::net::FetchError;
use census_domain::model::PreparedCaptureInputV1;
use census_store::{ArchiveWriteFence, RecoveryArchiveIo};

pub(in crate::net) fn restore_exact_prepared_held(
    fence: &ArchiveWriteFence,
    prepared: &PreparedCaptureInputV1,
    body: &[u8],
    capacity: u64,
    mut io: RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    super::diagnostic::admit_error_boxes(&mut io)?;
    let paths = paths::ArchivePaths::new(fence.root(), prepared, &mut io)?;
    let body_missing = shell::regular(&paths.body, &mut io)?.is_none();
    let metadata_missing = shell::regular(&paths.metadata, &mut io)?.is_none();
    paths::initialize(fence.root(), &paths, &mut io)?;
    let admission = Admission::begin(fence.root(), &paths, capacity, &mut io)?;
    let body_bytes = if body_missing {
        prepared.content_bytes()
    } else {
        0
    };
    let metadata_bytes = if metadata_missing {
        u64::try_from(prepared.metadata_bytes().len())
            .map_err(|_| shell::limit("metadata length"))?
    } else {
        0
    };
    let reservation = admission.reserve(
        prepared.content_digest().as_str(),
        prepared.metadata_digest().as_str(),
        (body_bytes, metadata_bytes),
        &mut io,
    )?;
    let stage = stage::create(fence.root(), &mut io)?;
    let result = restore_pair(
        &paths,
        &stage,
        prepared,
        body,
        (body_missing, metadata_missing),
        &mut io,
    );
    let cleanup = reservations::remove(&reservation, &mut io);
    let result = super::diagnostic::with_cleanup(&stage, result, cleanup, &mut io);
    match result {
        Ok(()) => stage::cleanup(&stage, &["new.body", "new.meta.json"], &mut io),
        Err(error) => stage::fail(&stage, error, &mut io),
    }
}

fn restore_pair(
    paths: &paths::ArchivePaths,
    stage: &std::path::Path,
    prepared: &PreparedCaptureInputV1,
    body: &[u8],
    missing: (bool, bool),
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(), FetchError> {
    if missing.0 {
        let leaf = stage::write_leaf(stage, "new.body", body, io)?;
        stage::publish(&leaf, &paths.body, io)?;
        io.release_owned(leaf.as_os_str().len())?;
    }
    if missing.1 {
        let leaf = stage::write_leaf(stage, "new.meta.json", prepared.metadata_bytes(), io)?;
        stage::publish(&leaf, &paths.metadata, io)?;
        io.release_owned(leaf.as_os_str().len())?;
    }
    Ok(())
}
