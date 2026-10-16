use super::run;
use crate::milesplit::RetainedJournalView;
use crate::milesplit::{
    read_retained_owned_result_from, CaptureReadBudget, ResultSetRef, VerifiedRetainedOwnedResult,
    OWNED_CAPTURE_PHASE,
};
use census_store::{JournalReadBudget, StoreSnapshot};
use serde_json::Value;

mod chunks;
mod error;
mod facts;
mod manifest;
mod session;
mod types;
pub use session::RetainedResultSession;

pub use error::RetainedMetadataError;
pub use types::{
    RawMetadataArchiveRef, RetainedPerformanceDate, RetainedResultDivision,
    VerifiedRetainedResultMetadata,
};

pub fn read_retained_result_metadata(
    view: &StoreSnapshot<'_>,
    reference: &ResultSetRef,
    archive: &RawMetadataArchiveRef,
    result: &VerifiedRetainedOwnedResult,
    budget: CaptureReadBudget,
) -> Result<VerifiedRetainedResultMetadata, RetainedMetadataError> {
    read_retained_result_metadata_from(view, reference, archive, result, budget)
}

pub fn read_retained_result_metadata_from(
    view: &impl RetainedJournalView,
    reference: &ResultSetRef,
    archive: &RawMetadataArchiveRef,
    result: &VerifiedRetainedOwnedResult,
    budget: CaptureReadBudget,
) -> Result<VerifiedRetainedResultMetadata, RetainedMetadataError> {
    session::check_reference(reference)?;
    view.store_identity()?;
    let rebound = read_retained_owned_result_from(
        view,
        result.acquisition().reference(),
        &result.row().locator,
        budget,
    )?;
    RetainedResultSession::new(view, budget).bind(reference, archive, rebound)
}

fn read(
    view: &impl RetainedJournalView,
    key: &str,
    max: usize,
) -> Result<Value, RetainedMetadataError> {
    view.journal_payload_bounded(OWNED_CAPTURE_PHASE, key, JournalReadBudget::new(max)?)?
        .ok_or_else(|| RetainedMetadataError::Missing { key: key.into() })
}
