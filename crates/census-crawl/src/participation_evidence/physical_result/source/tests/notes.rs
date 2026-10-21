use super::*;
use serde_json::{json, Value};

#[test]
fn every_immutable_tfrrs_note_field_is_checked_against_original_projection() -> TestResult {
    let original = fixture::tfrrs()?;
    for field in [
        "list_context_reference",
        "context_reference",
        "original_row_reference",
        "original_row_digest",
        "published_mark",
        "original_mark",
        "conversion_note",
    ] {
        let changed = fixture::note(&original, |note| {
            let value = note.get_mut(field).ok_or("original immutable field")?;
            let before = value.as_str().ok_or("immutable text field")?;
            *value = json!(format!("{before}x"));
            Ok(())
        })?;
        fixture::refused(&changed, &original)?;
    }
    for field in ["published_mark_span", "conversion_note_span"] {
        let changed = fixture::note(&original, |note| {
            let start = note
                .get_mut(field)
                .and_then(Value::as_object_mut)
                .and_then(|span| span.get_mut("start"))
                .ok_or("exact mark span")?;
            *start = json!(start.as_u64().ok_or("original span start")? + 1);
            Ok(())
        })?;
        fixture::refused(&changed, &original)?;
    }
    for field in [
        "subject_reference",
        "proposed_athlete",
        "proposed_school",
        "locator",
    ] {
        let changed = fixture::note(&original, |note| {
            let target = note
                .get_mut("proposed_target")
                .and_then(Value::as_object_mut)
                .ok_or("retained target")?;
            let value = target.get_mut(field).ok_or("target immutable field")?;
            *value = if field == "locator" {
                json!({"ordinal":118,"byte_offset":205298,"byte_length":1409})
            } else {
                json!("changed")
            };
            Ok(())
        })?;
        fixture::refused(&changed, &original)?;
    }
    Ok(())
}

#[test]
fn tfrrs_review_references_must_remain_real_sha256_strings() -> TestResult {
    let original = fixture::tfrrs()?;
    for field in [
        "proposed_attachment_reference",
        "proposed_school_binding_reference",
    ] {
        for invalid in [
            json!(null),
            json!(7),
            json!(""),
            json!("g".repeat(64)),
            json!("a".repeat(63)),
        ] {
            let changed = fixture::note(&original, |note| {
                let target = note
                    .get_mut("proposed_target")
                    .and_then(Value::as_object_mut)
                    .ok_or("retained target")?;
                *target.get_mut(field).ok_or("review digest")? = invalid;
                Ok(())
            })?;
            fixture::refused(&changed, &original)?;
        }
    }
    Ok(())
}

#[test]
fn missing_unknown_and_malformed_tfrrs_note_material_is_refused() -> TestResult {
    let original = fixture::tfrrs()?;
    for field in [
        "original_mark",
        "conversion_note",
        "conversion_note_span",
        "proposed_target",
    ] {
        let changed = fixture::note(&original, |note| {
            note.remove(field).ok_or("required original note field")?;
            Ok(())
        })?;
        fixture::refused(&changed, &original)?;
    }
    for target in [false, true] {
        let changed = fixture::note(&original, |note| {
            let destination = if target {
                note.get_mut("proposed_target")
                    .and_then(Value::as_object_mut)
                    .ok_or("retained target")?
            } else {
                note
            };
            destination.insert("foreign_claim".into(), json!(true));
            Ok(())
        })?;
        fixture::refused(&changed, &original)?;
    }
    for field in ["published_mark_span", "conversion_note_span", "locator"] {
        let changed = fixture::note(&original, |note| {
            let nested = if field == "locator" {
                note.get_mut("proposed_target")
                    .and_then(Value::as_object_mut)
                    .and_then(|target| target.get_mut(field))
            } else {
                note.get_mut(field)
            };
            nested
                .and_then(Value::as_object_mut)
                .ok_or("original nested proof")?
                .insert("foreign_claim".into(), json!(true));
            Ok(())
        })?;
        fixture::refused(&changed, &original)?;
    }
    for malformed in ["", "{", "[]", "null"] {
        let mut changed = original.clone();
        changed
            .evidence
            .first_mut()
            .ok_or("retained evidence")?
            .note = Some(malformed.into());
        fixture::refused(&changed, &original)?;
    }
    Ok(())
}

#[test]
fn tfrrs_notes_have_the_existing_exact_8192_byte_limit() -> TestResult {
    let original = fixture::tfrrs()?;
    let note = original
        .evidence
        .first()
        .and_then(|evidence| evidence.note.as_deref())
        .ok_or("retained note")?;
    let padding = 8192usize
        .checked_sub(note.len())
        .ok_or("bounded original note")?;
    let mut exact = original.clone();
    exact.evidence.first_mut().ok_or("retained evidence")?.note =
        Some(format!("{note}{}", " ".repeat(padding)));
    check!(matches_physical_observation(&exact, &original)?);
    check!(matches_physical_observation(&original, &exact)?);
    let mut over = original.clone();
    over.evidence.first_mut().ok_or("retained evidence")?.note =
        Some(format!("{note}{}", " ".repeat(padding + 1)));
    check!(matches!(
        matches_physical_observation(&over, &original),
        Err(Error::Budget {
            needed: 8193,
            limit: 8192,
            ..
        })
    ));
    check!(matches!(
        matches_physical_observation(&original, &over),
        Err(Error::Budget {
            needed: 8193,
            limit: 8192,
            ..
        })
    ));
    Ok(())
}
