use super::run;
use crate::milesplit::{
    read_retained_owned_result, CaptureReadBudget, ResultSetRef, VerifiedRetainedOwnedResult,
    OWNED_CAPTURE_PHASE,
};
use census_store::{JournalReadBudget, StoreSnapshot};
use serde_json::Value;

mod chunks;
mod error;
mod facts;
mod manifest;
mod types;

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
    if reference.url.len() > 4096 || reference.meet_id.len() > 20 || reference.rsid.len() > 20 {
        return Err(error::invalid(
            "result-set reference exceeds its source envelope",
        ));
    }
    let store_identity = view.store_identity()?;
    let result = read_retained_owned_result(
        view,
        result.acquisition().reference(),
        &result.row().locator,
        budget,
    )?;
    if reference.meet_id.parse::<u64>().ok() != Some(result.row().meet_id)
        || reference.rsid.parse::<u64>().ok() != Some(result.row().result_set_id)
    {
        return Err(RetainedMetadataError::ForeignResult);
    }
    let (manifest, mut capture, acquired_at) = manifest::load(view, reference, archive, budget)?;
    capture.body = chunks::body(view, archive, &manifest, capture.bytes)?;
    if crate::net::cache::content_digest(&capture.body) != capture.content_digest {
        return Err(error::invalid(
            "reassembled body SHA256 differs from retained digest",
        ));
    }
    let (page, start, end) = run::metadata::parse_capture_with_dates(&capture, reference)?;
    let date = facts::date(&start, end.as_deref());
    let division = facts::division(&result.row().provider);
    Ok(VerifiedRetainedResultMetadata {
        store_identity,
        snapshot_sequence: view.sequence(),
        archive: archive.clone(),
        capture,
        acquired_at,
        page,
        result,
        date,
        division,
    })
}

fn read(view: &StoreSnapshot<'_>, key: &str, max: usize) -> Result<Value, RetainedMetadataError> {
    view.journal_payload_bounded(OWNED_CAPTURE_PHASE, key, JournalReadBudget::new(max)?)?
        .ok_or_else(|| RetainedMetadataError::Missing { key: key.into() })
}
