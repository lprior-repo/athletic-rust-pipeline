use census_domain::school_directory::IdentifiedKey;
use census_store::{StoreError, StoreResult, MAX_OBSERVATION_BATCH_BYTES};
use serde::Serialize;

use super::budget::check;
use super::{copy_optional, copy_string, digest, reserve, APPLICATION_VERSION};
use crate::school_address::join::{LaneEvidence, LaneSet};

#[derive(Serialize)]
pub(in crate::school_address::join) struct ApplicationScope {
    pub(super) digest: String,
    pub(super) provenance: ScopeProvenance,
}

#[derive(Serialize)]
pub(super) struct ScopeProvenance {
    application_version: &'static str,
    lanes: Vec<LaneReference>,
    providers: Vec<String>,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct LaneReference {
    capture_sha256: String,
    captured_digest: String,
    generation: String,
    observed_on: Option<String>,
    path: String,
    provider: String,
    url: Option<String>,
}

impl ApplicationScope {
    pub(in crate::school_address::join) fn new(lanes: &LaneSet) -> StoreResult<Self> {
        admit_scope(lanes)?;
        let mut providers = Vec::new();
        let mut references = Vec::new();
        for provider in lanes.tokens() {
            reserve(&mut providers, 1)?;
            providers.push(copy_string(provider)?);
            for lane in lanes.lanes(provider) {
                let reference = LaneReference::new(provider, lane)?;
                if references.contains(&reference) {
                    continue;
                }
                reserve(&mut references, 1)?;
                references.push(reference);
            }
        }
        references.sort_unstable();
        let provenance = ScopeProvenance {
            application_version: APPLICATION_VERSION,
            providers,
            lanes: references,
        };
        Ok(Self {
            digest: digest(&provenance)?,
            provenance,
        })
    }
}

impl LaneReference {
    fn new(provider: &str, lane: &LaneEvidence) -> StoreResult<Self> {
        let captured_digest = captured_digest(&lane.captured)?;
        Ok(Self {
            provider: copy_string(provider)?,
            path: copy_string(&lane.path)?,
            capture_sha256: copy_string(&lane.capture_sha256)?,
            generation: copy_string(&lane.generation)?,
            url: copy_optional(&lane.url)?,
            observed_on: copy_optional(&lane.observed_on)?,
            captured_digest,
        })
    }
}

fn captured_digest(captured: &[IdentifiedKey]) -> StoreResult<String> {
    let mut ordered = Vec::new();
    reserve(&mut ordered, captured.len())?;
    ordered.extend(captured.iter());
    ordered.sort_unstable();
    ordered.dedup();
    digest(&ordered)
}

fn admit_scope(lanes: &LaneSet) -> StoreResult<()> {
    let mut bytes = std::mem::size_of::<ApplicationScope>();
    admit_scope_bytes(&mut bytes, 64)?;
    for provider in lanes.tokens() {
        admit_scope_bytes(&mut bytes, std::mem::size_of::<String>())?;
        admit_scope_bytes(&mut bytes, provider.len())?;
        for lane in lanes.lanes(provider) {
            admit_scope_bytes(&mut bytes, std::mem::size_of::<LaneReference>())?;
            for text in [
                provider.as_str(),
                lane.path.as_str(),
                lane.capture_sha256.as_str(),
                lane.generation.as_str(),
                lane.url.as_deref().map_or("", |value| value),
                lane.observed_on.as_deref().map_or("", |value| value),
            ] {
                admit_scope_bytes(&mut bytes, text.len())?;
            }
            admit_scope_bytes(&mut bytes, 64)?;
            let scratch = lane
                .captured
                .len()
                .checked_mul(std::mem::size_of::<&IdentifiedKey>())
                .ok_or(StoreError::CounterOverflow)?;
            admit_scope_bytes(&mut bytes, scratch)?;
        }
    }
    Ok(())
}

fn admit_scope_bytes(held: &mut usize, bytes: usize) -> StoreResult<()> {
    let total = held.checked_add(bytes).ok_or(StoreError::CounterOverflow)?;
    check(
        "school-address application scope bytes",
        total,
        MAX_OBSERVATION_BATCH_BYTES,
    )?;
    *held = total;
    Ok(())
}
