use super::{acquisition_manifest_key, OWNED_CAPTURE_PHASE};
use crate::net::cache::content_digest;
use census_store::{JournalReadBudget, StoreSnapshot};
use serde::de::DeserializeOwned;
use serde_json::Value;

mod chunks;
mod error;
mod manifests;
mod types;

pub use error::CaptureWitnessError;
pub use types::{CaptureReadBudget, OwnedAcquisitionRef, VerifiedOwnedAcquisition};

const METADATA_JOURNAL_BYTES: usize = 16 * 1024;
const CHUNK_JOURNAL_BYTES: usize = 64 * 1024;

pub fn read_retained_acquisition(
    view: &StoreSnapshot<'_>,
    reference: &OwnedAcquisitionRef,
    budget: CaptureReadBudget,
) -> Result<VerifiedOwnedAcquisition, CaptureWitnessError> {
    let key = reference.manifest_key();
    let mut occurrence: manifests::Occurrence =
        decode(read(view, key, METADATA_JOURNAL_BYTES)?, key)?;
    manifests::validate_capture(&occurrence.capture, reference, key)?;
    let actual = acquisition_manifest_key(reference.meet_id(), &occurrence.capture)
        .map_err(|source| error::corrupt(key, format!("acquisition digest failed: {source}")))?;
    if actual != key {
        return Err(error::foreign(
            key,
            "acquisition metadata does not reproduce its manifest key",
        ));
    }
    let layout = occurrence.layout(reference, budget)?;
    let content: manifests::Content = decode(
        read(view, &occurrence.content_manifest, METADATA_JOURNAL_BYTES)?,
        &occurrence.content_manifest,
    )?;
    content.validate(reference, &occurrence, layout)?;
    occurrence.capture.body = chunks::read_body(view, reference, &occurrence, layout)?;
    if content_digest(&occurrence.capture.body) != occurrence.capture.content_digest {
        return Err(error::corrupt(
            key,
            "restored body SHA256 differs from retained content digest",
        ));
    }
    Ok(VerifiedOwnedAcquisition::new(
        reference.clone(),
        occurrence.capture,
    ))
}

fn read(view: &StoreSnapshot<'_>, key: &str, max: usize) -> Result<Value, CaptureWitnessError> {
    let budget = JournalReadBudget::new(max).map_err(|source| CaptureWitnessError::Store {
        key: key.into(),
        source: Box::new(source),
    })?;
    view.journal_payload_bounded(OWNED_CAPTURE_PHASE, key, budget)
        .map_err(|source| store_error(key, source))?
        .ok_or_else(|| CaptureWitnessError::Missing { key: key.into() })
}

fn decode<T: DeserializeOwned>(value: Value, key: &str) -> Result<T, CaptureWitnessError> {
    manifests::capture_shape(&value, key)?;
    serde_json::from_value(value)
        .map_err(|source| error::corrupt(key, format!("invalid retained entry shape: {source}")))
}

fn store_error(key: &str, source: census_store::StoreError) -> CaptureWitnessError {
    match source {
        census_store::StoreError::JournalReadLimit {
            what, needed, max, ..
        } => CaptureWitnessError::Budget {
            key: key.into(),
            what,
            needed,
            max,
        },
        source => CaptureWitnessError::Store {
            key: key.into(),
            source: Box::new(source),
        },
    }
}
