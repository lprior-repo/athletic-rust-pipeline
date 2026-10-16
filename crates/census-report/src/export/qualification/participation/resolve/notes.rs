use super::{invalid, note, Accumulated, Evidence, Gap, StoreResult, Value};
use std::collections::BTreeSet;

pub(super) fn read(
    row: &(Evidence, Value),
    metadata: &(Evidence, Value),
    result: &mut Accumulated,
    seen: &mut BTreeSet<(String, String, String)>,
) -> StoreResult<Option<note::SourceNotes>> {
    if row.1.get("raw_metadata_url").and_then(Value::as_str) != metadata.0.source.url.as_deref() {
        return Ok(None);
    }
    let notes = match note::SourceNotes::read(&row.0, &row.1, &metadata.0, &metadata.1) {
        Ok(notes) => notes,
        Err(_) => {
            result.gaps.insert(Gap::SourceBindingMismatch);
            return Ok(None);
        }
    };
    let key = (
        notes.owned.manifest_key().to_owned(),
        notes.locator.clone(),
        notes.raw.manifest_key().to_owned(),
    );
    result
        .references
        .insert(notes.owned.manifest_key().to_owned());
    result
        .references
        .insert(notes.raw.manifest_key().to_owned());
    if !seen.insert(key) {
        return Ok(None);
    }
    if seen.len() > 16 {
        return Err(invalid("participation source alternatives exceed 16"));
    }
    Ok(Some(notes))
}
