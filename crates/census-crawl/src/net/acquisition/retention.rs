use super::session::{AdoptedSession, CaptureSession, Stage};
use super::CaptureProvenance;
use crate::net::cache::{
    publish_eligible, write_archive, write_cache, CacheMeta, Capture, EligibleCacheV2,
};
use crate::net::{CaptureTransportError, FetchError, FetchOutcome};
use census_domain::model::PreparedCaptureInputV1;
use census_store::AcquisitionReadBudget;
use std::path::Path;

impl CaptureSession<'_> {
    pub(in crate::net) fn retain(
        &mut self,
        body_path: &Path,
        meta_path: &Path,
        body: &[u8],
        metadata: &CacheMeta,
    ) -> Result<(), FetchError> {
        #[cfg(test)]
        let interruption = self.interruption();
        let Self::Adopted(adopted) = self else {
            if metadata.status == 200 {
                write_cache(body_path, meta_path, body, metadata)?;
            } else {
                write_archive(body_path, meta_path, body, metadata)?;
            }
            return Ok(());
        };
        let AdoptedSession { store, stage, .. } = adopted.as_mut();
        let capture = Capture::from_meta(metadata, meta_path)?;
        capture.verify_body(body, body_path)?;
        let length =
            u64::try_from(body.len()).map_err(|_| CaptureTransportError::BackingUnavailable)?;
        let input = PreparedCaptureInputV1::new(&metadata.content_digest, length, capture.encoded)?;
        let response = match std::mem::replace(stage, Stage::Consumed) {
            Stage::Observed(response) => response,
            _ => return Err(CaptureTransportError::DispatchConsumed.into()),
        };
        let disposition =
            census_domain::model::CaptureDispositionV1::for_http_status(metadata.status)?;
        let prepared = store.prepare_acquisition_capture(response, input, disposition)?;
        #[cfg(test)]
        interruption.at(super::CaptureInterruption::AfterPrepared)?;
        let archived_body = write_archive(body_path, meta_path, body, metadata)?;
        #[cfg(test)]
        interruption.at(super::CaptureInterruption::AfterArchive)?;
        let occurrence = store.finish_acquisition_capture(prepared)?;
        *stage = Stage::Complete(CaptureProvenance::retained(
            occurrence.clone(),
            occurrence.clone(),
        ));
        if metadata.status == 200 {
            let envelope = EligibleCacheV2::new(metadata.clone(), occurrence);
            publish_eligible(body_path, meta_path, &envelope, &archived_body)?;
        }
        Ok(())
    }

    pub(in crate::net) fn replay_not_modified(
        &mut self,
        body_path: &Path,
        meta_path: &Path,
    ) -> Result<Option<FetchOutcome>, FetchError> {
        let Self::Adopted(adopted) = self else {
            return Ok(None);
        };
        let AdoptedSession {
            store,
            stage,
            cached,
            ..
        } = adopted.as_mut();
        let pinned = cached
            .as_ref()
            .ok_or(CaptureTransportError::CacheOccurrenceMismatch)?;
        let mut budget = AcquisitionReadBudget::for_work();
        let (result, body) = pinned.compare_body(body_path, meta_path, |body| {
            let response = match std::mem::replace(stage, Stage::Consumed) {
                Stage::Observed(response) => response,
                _ => return Err(CaptureTransportError::DispatchConsumed.into()),
            };
            let reference = store.finish_acquisition_not_modified(
                response,
                &pinned.envelope.original_occurrence,
                &mut budget,
            )?;
            Ok((reference, body))
        })?;
        let meta = &pinned.envelope.metadata;
        *stage = Stage::Complete(CaptureProvenance::retained(
            result,
            pinned.envelope.original_occurrence.clone(),
        ));
        Ok(Some(FetchOutcome {
            url: meta.url.clone(),
            response_url: meta.response_url.clone(),
            method: meta.method.clone(),
            status: meta.status,
            content_digest: meta.content_digest.clone(),
            bytes: body.len(),
            fetched_at: meta.fetched_at.clone(),
            from_cache: false,
            content_type: meta.content_type.clone(),
            body,
        }))
    }
}
