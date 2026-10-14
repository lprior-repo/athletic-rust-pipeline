use super::archive::{admission, ArchiveAdmission, ArchiveLock};
use super::archive_io::{invalid, lock_cache, present, replace_cache, with_stage};
use super::envelope::{decode, CacheDocument, EligibleCacheV2};
use super::{
    content_digest, read_cache_file, CacheMeta, Capture, FetchError, MAX_BODY_BYTES, MAX_META_BYTES,
};
use crate::net::{CaptureTransportError, RepresentationHeaders};
use census_store::{AcquisitionReadBudget, Store, VerifiedRetainedOccurrence};
use std::path::Path;

pub(crate) struct PinnedEligibleCache {
    pub(crate) envelope: EligibleCacheV2,
    encoded: Vec<u8>,
}

pub(crate) fn read_eligible(
    body_path: &Path,
    meta_path: &Path,
    url: &str,
    representation: &RepresentationHeaders,
) -> Result<Option<PinnedEligibleCache>, FetchError> {
    match (present(body_path)?, present(meta_path)?) {
        (false, false) => return Ok(None),
        (true, true) => {}
        _ => return Err(CaptureTransportError::CacheOccurrenceMismatch.into()),
    }
    let encoded = read_cache_file(meta_path, MAX_META_BYTES, None)?
        .ok_or(CaptureTransportError::CacheEnvelopeUnsupported)?;
    let envelope = match decode(&encoded, meta_path)? {
        CacheDocument::Historical(_) => {
            return Err(CaptureTransportError::HistoricalAttemptUnknown.into())
        }
        CacheDocument::Eligible(envelope) => envelope,
    };
    let meta = &envelope.metadata;
    if meta.method != "GET"
        || meta.url != url
        || &meta.representation != representation
        || meta.status != 200
    {
        return Err(CaptureTransportError::CacheOccurrenceMismatch.into());
    }
    Capture::from_meta(meta, meta_path)?;
    verify_body(body_path, meta)?;
    Ok(Some(PinnedEligibleCache { envelope, encoded }))
}

fn verify_body(path: &Path, meta: &CacheMeta) -> Result<Vec<u8>, FetchError> {
    let body = read_cache_file(path, MAX_BODY_BYTES, Some(meta.bytes))?
        .ok_or(CaptureTransportError::CacheOccurrenceMismatch)?;
    if content_digest(&body) != meta.content_digest {
        return Err(CaptureTransportError::CacheOccurrenceMismatch.into());
    }
    Ok(body)
}

impl PinnedEligibleCache {
    pub(crate) fn compare<T>(
        &self,
        body_path: &Path,
        meta_path: &Path,
        action: impl FnOnce() -> Result<T, FetchError>,
    ) -> Result<T, FetchError> {
        self.compare_body(body_path, meta_path, |_| action())
    }

    pub(crate) fn compare_body<T>(
        &self,
        body_path: &Path,
        meta_path: &Path,
        action: impl FnOnce(Vec<u8>) -> Result<T, FetchError>,
    ) -> Result<T, FetchError> {
        let _lock = lock_cache(meta_path)?;
        let current = read_cache_file(meta_path, MAX_META_BYTES, Some(self.encoded.len()))?
            .ok_or(CaptureTransportError::CacheOccurrenceMismatch)?;
        if current != self.encoded {
            return Err(CaptureTransportError::CacheOccurrenceMismatch.into());
        }
        let body = verify_body(body_path, &self.envelope.metadata)?;
        action(body)
    }

    pub(crate) fn verify(
        &self,
        store: &Store,
        budget: &mut AcquisitionReadBudget,
        meta_path: &Path,
    ) -> Result<VerifiedRetainedOccurrence, FetchError> {
        let retained =
            store.read_retained_acquisition_v1(&self.envelope.original_occurrence, budget)?;
        self.require_retained(&retained, meta_path)?;
        Ok(retained)
    }

    pub(crate) fn require_retained(
        &self,
        retained: &VerifiedRetainedOccurrence,
        meta_path: &Path,
    ) -> Result<(), FetchError> {
        if retained.revalidation().is_some()
            || retained.body_reference() != &self.envelope.original_occurrence
        {
            return Err(CaptureTransportError::CacheOccurrenceMismatch.into());
        }
        let capture = Capture::from_meta(&self.envelope.metadata, meta_path)?;
        if capture.encoded != retained.prepared().metadata_bytes() {
            return Err(CaptureTransportError::CacheOccurrenceMismatch.into());
        }
        Ok(())
    }
}

pub(crate) fn publish_eligible(
    body_path: &Path,
    meta_path: &Path,
    envelope: &EligibleCacheV2,
    archived_body: &Path,
) -> Result<(), FetchError> {
    with_archive_eligible_fence(body_path, meta_path, |archive_lock| {
        publish_eligible_held(archive_lock, body_path, meta_path, envelope, archived_body)
    })
}

pub(in crate::net) fn publish_eligible_held(
    archive_lock: &ArchiveLock,
    body_path: &Path,
    meta_path: &Path,
    envelope: &EligibleCacheV2,
    archived_body: &Path,
) -> Result<(), FetchError> {
    let root = archive_lock.root();
    if super::archive::cache_root(body_path, meta_path)? != root {
        return Err(invalid(
            meta_path,
            "cache paths do not belong to the held archive root",
        ));
    }
    let encoded = envelope.encode(meta_path)?;
    with_stage(root, |stage| {
        let archive = super::archive::initialize_archive(root)?;
        let mut admission =
            ArchiveAdmission::begin_with_capacity(archive_lock, admission::archive_capacity()?)?;
        super::archive::preserve_eligible_previous(
            &archive,
            stage,
            body_path,
            meta_path,
            &mut admission,
        )?;
        replace_cache(stage, archived_body, body_path, meta_path, &encoded)
    })
}

pub(in crate::net) fn with_archive_eligible_fence<T>(
    body_path: &Path,
    meta_path: &Path,
    action: impl FnOnce(&ArchiveLock) -> Result<T, FetchError>,
) -> Result<T, FetchError> {
    let root = super::archive::cache_root(body_path, meta_path)?;
    let archive_lock = ArchiveLock::acquire(root)?;
    let _eligible_lock = lock_cache(meta_path)?;
    #[cfg(test)]
    super::archive::lock_tests::checkpoint(
        &root.join("archive-capacity.lock"),
        super::archive::lock_tests::LockEvent::EligibleAcquired,
    )?;
    action(&archive_lock)
}
