use super::{metadata, CaptureProvenanceView, CapturedFetch};
use crate::net::{CaptureTransportError, FetchError, FetchOutcome};
use census_domain::model::{AcquisitionOccurrenceRefV1, NormalizedObservedHttpV1};

pub struct RetainedObservedCaptureView<'a> {
    capture: &'a FetchOutcome,
    occurrence: &'a AcquisitionOccurrenceRefV1,
    original: &'a AcquisitionOccurrenceRefV1,
    original_metadata: &'a [u8],
    observed_http: NormalizedObservedHttpV1,
}

impl RetainedObservedCaptureView<'_> {
    pub fn capture(&self) -> &FetchOutcome {
        self.capture
    }

    pub fn occurrence(&self) -> &AcquisitionOccurrenceRefV1 {
        self.occurrence
    }

    pub fn original(&self) -> &AcquisitionOccurrenceRefV1 {
        self.original
    }

    pub fn original_metadata(&self) -> &[u8] {
        self.original_metadata
    }

    pub fn observed_http(&self) -> &NormalizedObservedHttpV1 {
        &self.observed_http
    }
}

impl CapturedFetch {
    pub fn retained_observed(&self) -> Result<Option<RetainedObservedCaptureView<'_>>, FetchError> {
        let Some(original_metadata) = self.original_metadata.as_deref() else {
            return Ok(None);
        };
        let Some(observed_http) = metadata::observed(original_metadata, &self.capture)? else {
            return Ok(None);
        };
        let CaptureProvenanceView::Retained {
            occurrence,
            original,
        } = self.provenance()
        else {
            return Err(CaptureTransportError::BackingUnavailable.into());
        };
        if occurrence.attempt().store_identity() != original.attempt().store_identity()
            || observed_http
                .hops()
                .iter()
                .any(|hop| hop.dispatch().attempt() != original.attempt())
        {
            return Err(CaptureTransportError::BackingUnavailable.into());
        }
        Ok(Some(RetainedObservedCaptureView {
            capture: &self.capture,
            occurrence,
            original,
            original_metadata,
            observed_http,
        }))
    }
}
