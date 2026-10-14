use super::{CaptureProvenance, CapturedFetch};
use crate::net::cache::{CacheMeta, Capture, EligibleCacheV2};
use crate::net::{CaptureTransportError, FetchError, FetchOutcome, Fetcher};
use census_domain::model::AcquisitionOccurrenceRefV1;
use census_store::{
    AcquisitionReadBudget, OwnedAcquisitionWork, Store, VerifiedRetainedOccurrence,
};

pub(super) fn replay(
    store: &Store,
    reference: &AcquisitionOccurrenceRefV1,
) -> Result<CapturedFetch, FetchError> {
    let mut budget = AcquisitionReadBudget::for_work();
    outcome(
        store.read_retained_acquisition_v1(reference, &mut budget)?,
        true,
    )
}

pub(super) fn outcome(
    retained: VerifiedRetainedOccurrence,
    from_cache: bool,
) -> Result<CapturedFetch, FetchError> {
    let metadata: CacheMeta = serde_json::from_slice(retained.prepared().metadata_bytes())
        .map_err(|source| FetchError::Decode {
            target: retained.body_path().display().to_string(),
            source,
        })?;
    let body = super::super::cache::read_cache_file(
        retained.body_path(),
        super::super::MAX_BODY_BYTES,
        Some(metadata.bytes),
    )?
    .ok_or(CaptureTransportError::BackingUnavailable)?;
    Capture::from_meta(&metadata, retained.body_path())?
        .verify_body(&body, retained.body_path())?;
    let provenance = CaptureProvenance::Retained {
        occurrence: retained.reference().clone(),
        original: retained.body_reference().clone(),
    };
    Ok(CapturedFetch {
        capture: FetchOutcome {
            url: metadata.url,
            response_url: metadata.response_url,
            method: metadata.method,
            status: metadata.status,
            content_digest: metadata.content_digest,
            bytes: body.len(),
            fetched_at: metadata.fetched_at,
            from_cache,
            content_type: metadata.content_type,
            body,
        },
        provenance,
    })
}

impl Fetcher {
    pub(super) fn recover_prepared(
        &self,
        store: &Store,
        work: OwnedAcquisitionWork,
    ) -> Result<CapturedFetch, FetchError> {
        let url = work.work().request().original_url();
        let representation = crate::net::RepresentationHeaders::from_entries(
            work.work().request().representation_headers().to_vec(),
        )?;
        let extra = super::super::execute::representation::cache_extra(None, &representation);
        let (body_path, meta_path) = self.cache_paths(&Self::key_for("GET", url, &extra));
        crate::net::cache::with_archive_eligible_fence(&body_path, &meta_path, |archive_lock| {
            let mut finish_budget = AcquisitionReadBudget::for_work();
            let reference = store.finish_prepared_acquisition(work, &mut finish_budget)?;
            let mut read_budget = AcquisitionReadBudget::for_work();
            let retained = store.read_retained_acquisition_v1(&reference, &mut read_budget)?;
            let metadata: CacheMeta = serde_json::from_slice(retained.prepared().metadata_bytes())
                .map_err(|source| FetchError::Decode {
                    target: meta_path.display().to_string(),
                    source,
                })?;
            let captured = outcome(retained, true)?;
            let CaptureProvenance::Retained { original, .. } = &captured.provenance else {
                return Err(CaptureTransportError::CacheOccurrenceMismatch.into());
            };
            if captured.capture.status == 200 {
                let envelope = EligibleCacheV2::new(metadata, original.clone());
                let archived = self
                    .cache_dir
                    .join("archive/bodies")
                    .join(format!("{}.body", captured.capture.content_digest));
                crate::net::cache::publish_eligible_held(
                    archive_lock, &body_path, &meta_path, &envelope, &archived,
                )?;
            }
            Ok(captured)
        })
    }
}
