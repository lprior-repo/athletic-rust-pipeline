use super::super::{Accumulator, RESULT_SET_PHASE};
use super::acquired::AcquiredMeet;
use super::capture;
use super::{RawPage, ResultSetRef};
use crate::milesplit::owned::OwnedMeetVerdict;
use crate::net::FetchOutcome;
use crate::CrawlResult;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub(super) const PARSER_REVISION: &str = "milesplit_owned_v3_raw_metadata_v1";

pub(super) fn projection(
    reference: &ResultSetRef,
    page: &RawPage,
    metadata: &FetchOutcome,
    acquired: &AcquiredMeet,
    projected: &Accumulator,
    complete: bool,
    projected_rows: usize,
) -> CrawlResult<(String, serde_json::Value)> {
    let (completeness, ownership_complete) = match &acquired.outcome.verdict {
        OwnedMeetVerdict::Parsed(page) => (Some(page.completeness), page.ownership_complete()),
        _ => (None, false),
    };
    let payload = serde_json::json!({
        "meet": reference.meet_id, "rsid": reference.rsid,
        "source_url": reference.url, "source_instance": reference.site.source_id(),
        "jurisdiction": reference.site.jurisdiction(),
        "capture": capture::provenance(&acquired.outcome.capture),
        "raw_metadata_capture": capture::provenance(metadata),
        "raw_metadata_archive": capture::metadata_provenance(reference, metadata)?,
        "owned_capture_archive": crate::milesplit::owned::acquisition_manifest_key(crate::milesplit::fetch::owned_meet_id(reference)?, &acquired.outcome.capture)?,
        "parser_revision": PARSER_REVISION,
        "schema_revision": RESULT_SET_PHASE,
        "projection_context_digest": projected_digest(projected)?,
        "disposition": if complete { "projection_applied" } else { "partial" },
        "projected_rows": projected_rows, "raw_rejections": page.skipped,
        "raw_grade_issues": page.grade_issues, "raw_rows_role": "metadata only",
        "source_completeness": completeness, "ownership_complete": ownership_complete,
        "canonical_identity_accepted": false, "lifetime_pr_claimed": false,
        "census_sealed": false,
    });
    identified(reference, &payload).map(|key| (key, payload))
}

pub(super) fn failure(
    reference: &ResultSetRef,
    acquired: &AcquiredMeet,
    metadata: Option<&FetchOutcome>,
    reason: &str,
) -> CrawlResult<(String, serde_json::Value)> {
    let payload = serde_json::json!({
        "meet": reference.meet_id, "rsid": reference.rsid, "source_url": reference.url,
        "source_instance": reference.site.source_id(), "jurisdiction": reference.site.jurisdiction(),
        "capture": capture::provenance(&acquired.outcome.capture),
        "raw_metadata_capture": metadata.map(capture::provenance),
        "raw_metadata_archive": metadata.map(|capture| capture::metadata_provenance(reference, capture)).transpose()?,
        "owned_capture_archive": crate::milesplit::owned::acquisition_manifest_key(crate::milesplit::fetch::owned_meet_id(reference)?, &acquired.outcome.capture)?,
        "parser_revision": PARSER_REVISION, "schema_revision": RESULT_SET_PHASE,
        "disposition": "metadata_unavailable", "document_rejection": reason,
        "source_observations": "retained in owned capture/interpretation journals",
        "ownership_complete": false,
    });
    identified(reference, &payload).map(|key| (key, payload))
}

fn identified(reference: &ResultSetRef, payload: &serde_json::Value) -> CrawlResult<String> {
    let disposition = if payload["disposition"] == "projection_applied" {
        "projection"
    } else {
        "partial"
    };
    Ok(format!(
        "{disposition}/{}/{}/{}",
        reference.meet_id,
        reference.rsid,
        capture::digest(payload)?
    ))
}

fn projected_digest(projected: &Accumulator) -> CrawlResult<String> {
    capture::digest(&(
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
    projected.retained = std::mem::take(&mut projected.retained)
        .into_iter()
        .map(|(mut key, payload)| {
            let digest = capture::digest(&payload)?;
            key.push('/');
            key.push_str(&digest);
            Ok((key, payload))
        })
        .collect::<CrawlResult<_>>()?;
    Ok(())
}

#[cfg(test)]
mod tests;
