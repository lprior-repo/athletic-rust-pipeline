use census_crawl::participation_evidence::recorded::discover_current_html_results;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{EvidenceSubjectAttachmentStoreError, Store, StoreError};
use serde_json::{json, Value};

pub(super) fn inspect(store: &Store) -> Result<Value, Box<dyn std::error::Error>> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let plan = discover_current_html_results(&current, &mut grant)?;
    let mut found = None;
    for group in 0..plan.group_count() {
        plan.with_group(group, &mut grant, |linked, grant| {
            for index in 0..linked.row_count() {
                let Some(label) = linked.checked_event_label(index, grant)? else {
                    continue;
                };
                if label.person().id != "14570824" {
                    continue;
                }
                let row = linked.row(index).ok_or_else(|| {
                    StoreError::from(EvidenceSubjectAttachmentStoreError::Corrupt(
                        "checked event source row disappeared",
                    ))
                })?;
                let value = json!({
                    "source_person": label.person(),
                    "public_id": row.attachment().target().public_id,
                    "candidate": row.attachment().target().canonical_candidate,
                    "label": label.label(),
                    "source_url": label.source_url(),
                    "heading_capture": label.heading_proof().capture(),
                    "heading_locator": label.heading_proof().locator(),
                    "table_locator": label.table_proof().locator(),
                    "row_locator": label.row_proof().locator(),
                    "heading_reference": label.heading_reference(),
                    "row_reference": label.row_reference(),
                    "original_event_heading": row.checked_row().event().label,
                    "original_result_row": row.checked_row().reference(),
                    "original_mark": row.checked_row().parsed_row().mark,
                    "competition_day": linked.checked_document().published_start(),
                });
                if found.as_ref().is_some_and(|original| original != &value) {
                    return Err(
                        StoreError::from(EvidenceSubjectAttachmentStoreError::Corrupt(
                            "genuine event source ownership is ambiguous",
                        ))
                        .into(),
                    );
                }
                found = Some(value);
            }
            Ok(())
        })?;
    }
    found.ok_or_else(|| "genuine Owen checked event source is unavailable".into())
}
