use super::note;
use census_crawl::milesplit::{OwnedPerformance, VerifiedRetainedResultMetadata};
use census_domain::model::{Evidence, SourceRef};
use serde_json::{json, Value};

pub(super) fn owned_evidence(
    source: &VerifiedRetainedResultMetadata,
    notes: &note::SourceNotes,
) -> Evidence {
    let capture = source.result().acquisition().capture();
    let mut evidence = Evidence::parsed(
        SourceRef::new(notes.reference.site.source_id(), Some(capture.url.clone())),
        &capture.fetched_at,
    );
    evidence.note = Some(owned_note(source, notes, source.result().row()).to_string());
    evidence
}

pub(super) fn owned_note(
    source: &VerifiedRetainedResultMetadata,
    notes: &note::SourceNotes,
    row: &OwnedPerformance,
) -> Value {
    let capture = source.result().acquisition().capture();
    let page = source.page();
    json!({
        "locator": row.locator, "result_id": row.result_id, "meet_id": row.meet_id,
        "result_set_id": row.result_set_id, "team_id": row.team_id,
        "source_athlete": row.source_athlete, "cohort": row.cohort,
        "published_grad_year": row.grad_year, "provider": row.provider,
        "capture_url": capture.url, "sha256": capture.content_digest,
        "acquired_at": capture.fetched_at, "raw_metadata_url": notes.reference.url,
        "meet_name": page.meet.name, "meet_date": page.meet.date,
        "sport": page.sport, "school_year": page.school_year,
        "identity_decision": "none; source-owned observation only",
        "best_mark_scope": "observed performance; not lifetime PR",
    })
}

pub(super) fn known_provider(provider: &Value) -> bool {
    provider.as_object().is_some_and(|fields| {
        fields.keys().all(|field| {
            census_crawl::milesplit::OWNED_FIELDS
                .split(',')
                .any(|known| known == field)
        })
    })
}
