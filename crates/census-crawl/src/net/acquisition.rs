use super::{FetchError, FetchOptions, FetchOutcome, Fetcher};
use census_domain::model::{AcquisitionOccurrenceRefV1, RequestRepresentationV1};

mod metadata;
mod normalized;
mod observation;
mod public_resources;
mod recovery;
mod request;
mod retained;
mod transport;
pub use public_resources::{
    assess_directory_roster_child, CapturedPublicDirectory, CheckedDirectoryRosterWitnessV1,
    PublicResourceAcquisition,
};
pub use recovery::{EligiblePublicationOutcome, EligiblePublicationSkip, PreparedRecoveryOutcome};
pub use retained::RetainedObservedCaptureView;
mod read;
mod retention;
mod session;
#[cfg(test)]
mod testing;
#[cfg(test)]
pub(super) use testing::CaptureInterruption;
#[cfg(test)]
pub(crate) mod tests;
pub(super) use session::CaptureSession;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureProvenanceView<'a> {
    HistoricalAttemptUnknown,
    Retained {
        occurrence: &'a AcquisitionOccurrenceRefV1,
        original: &'a AcquisitionOccurrenceRefV1,
    },
}

#[derive(Debug)]
pub(crate) struct CaptureProvenance {
    state: CaptureProvenanceState,
}

#[derive(Debug)]
enum CaptureProvenanceState {
    HistoricalAttemptUnknown,
    Retained {
        occurrence: AcquisitionOccurrenceRefV1,
        original: AcquisitionOccurrenceRefV1,
    },
}

impl CaptureProvenance {
    fn historical() -> Self {
        Self {
            state: CaptureProvenanceState::HistoricalAttemptUnknown,
        }
    }

    fn retained(
        occurrence: AcquisitionOccurrenceRefV1,
        original: AcquisitionOccurrenceRefV1,
    ) -> Self {
        Self {
            state: CaptureProvenanceState::Retained {
                occurrence,
                original,
            },
        }
    }

    pub(crate) fn view(&self) -> CaptureProvenanceView<'_> {
        match &self.state {
            CaptureProvenanceState::HistoricalAttemptUnknown => {
                CaptureProvenanceView::HistoricalAttemptUnknown
            }
            CaptureProvenanceState::Retained {
                occurrence,
                original,
            } => CaptureProvenanceView::Retained {
                occurrence,
                original,
            },
        }
    }
}

#[derive(Debug)]
pub struct CapturedFetch {
    capture: FetchOutcome,
    provenance: CaptureProvenance,
    original_metadata: Option<Vec<u8>>,
}

impl CapturedFetch {
    pub(crate) fn historical(capture: FetchOutcome) -> Self {
        Self {
            capture,
            provenance: CaptureProvenance::historical(),
            original_metadata: None,
        }
    }

    pub(crate) fn accepted(self, options: &FetchOptions) -> Result<Self, FetchError> {
        accepted(self, options)
    }

    pub fn capture(&self) -> &FetchOutcome {
        &self.capture
    }

    pub fn provenance(&self) -> CaptureProvenanceView<'_> {
        self.provenance.view()
    }
    pub(crate) fn validate_observed_metadata(
        metadata: &[u8],
        capture: &FetchOutcome,
    ) -> Result<Option<census_domain::model::NormalizedObservedHttpV1>, FetchError> {
        metadata::observed(metadata, capture)
    }

    pub fn observed_http(
        &self,
    ) -> Result<Option<census_domain::model::NormalizedObservedHttpV1>, FetchError> {
        self.original_metadata
            .as_deref()
            .map(|bytes| metadata::observed(bytes, &self.capture))
            .transpose()
            .map(Option::flatten)
    }

    pub fn into_capture(self) -> FetchOutcome {
        self.capture
    }

    pub(crate) fn into_parser_parts(self) -> (FetchOutcome, CaptureProvenance) {
        (self.capture, self.provenance)
    }
}

impl From<census_store::CaptureError> for FetchError {
    fn from(value: census_store::CaptureError) -> Self {
        Self::from(census_store::StoreError::from(value))
    }
}

fn accepted(captured: CapturedFetch, options: &FetchOptions) -> Result<CapturedFetch, FetchError> {
    if captured.capture.status == 200 || (captured.capture.status == 404 && options.allow_not_found)
    {
        Ok(captured)
    } else {
        Err(FetchError::Http {
            status: captured.capture.status,
            url: captured.capture.url,
        })
    }
}
