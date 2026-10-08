use super::super::{Accumulator, RESULT_SET_PHASE};
use super::acquired::AcquiredMeet;
use super::capture;
use super::projection::Input;
use super::ResultSetRef;
use crate::milesplit::owned::OwnedMeetVerdict;
use crate::net::FetchOutcome;
use crate::{CrawlError, CrawlResult};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub(in crate::milesplit::results) const PARSER_REVISION: &str =
    "milesplit_owned_v3_raw_metadata_v1";

pub(super) fn projection(
    input: &Input<'_>,
    metadata: &FetchOutcome,
    projected: &Accumulator,
    complete: bool,
    rows: usize,
) -> CrawlResult<(String, serde_json::Value)> {
    let mut payload = context(input, metadata)?;
    let fields = payload.as_object_mut().ok_or_else(not_object)?;
    fields.insert(
        "projection_context_digest".into(),
        serde_json::json!(projected_digest(projected, input.performance_as_of)?),
    );
    fields.insert(
        "disposition".into(),
        serde_json::json!(if complete {
            "projection_applied"
        } else {
            "partial"
        }),
    );
    fields.insert("projected_rows".into(), serde_json::json!(rows));
    identified(input.reference, &payload).map(|key| (key, payload))
}

pub(super) fn context(
    input: &Input<'_>,
    metadata: &FetchOutcome,
) -> CrawlResult<serde_json::Value> {
    let reference = input.reference;
    let (completeness, ownership_complete) = match &input.acquired.outcome.verdict {
        OwnedMeetVerdict::Parsed(page) => (Some(page.completeness), page.ownership_complete()),
        _ => (None, false),
    };
    Ok(serde_json::json!({
        "meet": reference.meet_id, "rsid": reference.rsid, "source_url": reference.url,
        "performance_as_of": input.performance_as_of,
        "source_instance": reference.site.source_id(), "jurisdiction": reference.site.jurisdiction(),
        "capture": capture::provenance(&input.acquired.outcome.capture),
        "raw_metadata_capture": capture::provenance(metadata),
        "raw_metadata_archive": capture::metadata_provenance(reference, metadata)?,
        "owned_capture_archive": crate::milesplit::owned::acquisition_manifest_key(crate::milesplit::fetch::owned_meet_id(reference)?, &input.acquired.outcome.capture)?,
        "parser_revision": PARSER_REVISION, "schema_revision": RESULT_SET_PHASE,
        "raw_rejections": input.page.skipped, "raw_grade_issues": input.page.grade_issues,
        "raw_rows_role": "metadata only", "source_completeness": completeness,
        "ownership_complete": ownership_complete, "canonical_identity_accepted": false,
        "lifetime_pr_claimed": false, "census_sealed": false,
    }))
}

pub(super) fn failure(
    reference: &ResultSetRef,
    acquired: &AcquiredMeet,
    metadata: Option<&FetchOutcome>,
    reason: &str,
    performance_as_of: chrono::NaiveDate,
) -> CrawlResult<(String, serde_json::Value)> {
    let payload = serde_json::json!({
        "meet": reference.meet_id, "rsid": reference.rsid, "source_url": reference.url,
        "performance_as_of": performance_as_of,
        "source_instance": reference.site.source_id(), "jurisdiction": reference.site.jurisdiction(),
        "capture": capture::provenance(&acquired.outcome.capture),
        "raw_metadata_capture": metadata.map(capture::provenance),
        "raw_metadata_archive": metadata.map(|capture| capture::metadata_provenance(reference, capture)).transpose()?,
        "owned_capture_archive": crate::milesplit::owned::acquisition_manifest_key(crate::milesplit::fetch::owned_meet_id(reference)?, &acquired.outcome.capture)?,
        "parser_revision": PARSER_REVISION, "schema_revision": RESULT_SET_PHASE,
        "disposition": "metadata_unavailable", "document_rejection": reason,
        "source_observations": "retained in owned capture/interpretation journals", "ownership_complete": false,
    });
    identified(reference, &payload).map(|key| (key, payload))
}

pub(super) fn identified(
    reference: &ResultSetRef,
    payload: &serde_json::Value,
) -> CrawlResult<String> {
    let disposition = match payload["disposition"].as_str() {
        Some("projection_applied") => "projection",
        Some("window_applied") => "window",
        _ => "partial",
    };
    Ok(format!(
        "{disposition}/{}/{}/{}",
        reference.meet_id,
        reference.rsid,
        capture::digest(payload)?
    ))
}

fn projected_digest(
    projected: &Accumulator,
    performance_as_of: chrono::NaiveDate,
) -> CrawlResult<String> {
    capture::digest(&(
        &performance_as_of,
        ordered(&projected.meets),
        ordered(&projected.events),
        ordered(&projected.teams),
        ordered(&projected.athletes),
        ordered(&projected.performances),
        ordered(&projected.observations),
        projected.retained.keys().collect::<BTreeSet<_>>(),
    ))
}

fn ordered<T: Serialize>(rows: &HashMap<String, T>) -> BTreeMap<&str, &T> {
    rows.iter()
        .map(|(key, value)| (key.as_str(), value))
        .collect()
}

pub(super) fn identify_retained(projected: &mut Accumulator) -> CrawlResult<()> {
    let mut identified = HashMap::new();
    identified
        .try_reserve(projected.retained.len())
        .map_err(super::super::budget::reserve)?;
    std::mem::take(&mut projected.retained)
        .into_iter()
        .try_for_each(|(mut key, payload)| {
            let digest = capture::digest(&payload)?;
            let growth = digest.len().checked_add(1).ok_or_else(not_object)?;
            key.try_reserve(growth)
                .map_err(super::super::budget::reserve)?;
            key.push('/');
            key.push_str(&digest);
            identified.insert(key, payload);
            Ok::<_, CrawlError>(())
        })?;
    projected.retained = identified;
    Ok(())
}

fn not_object() -> CrawlError {
    CrawlError::Invariant {
        detail: "invalid result receipt object or length".into(),
    }
}

#[cfg(test)]
mod tests;
