use super::*;
use crate::evidence_subject_attachment::frozen::archive::{self, Archive, Row};

fn mutated(
    bundle: &Bundle,
    edit: impl FnOnce(&mut Archive<'_>) -> TestResult,
) -> TestResult<Bundle> {
    let mut archive = archive::read(bundle.bytes(), &mut AttachmentReplayGrantV1::new())?;
    edit(&mut archive)?;
    let raw = serde_json::to_vec(&archive)?;
    let expected = reference(&raw)?;
    Ok(Bundle::from_owned_bytes(
        raw,
        expected,
        &mut AttachmentReplayGrantV1::new(),
    )?)
}

pub(super) fn refused(bundle: &Bundle, expected: &'static str) -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(FrozenSource::read(bundle, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(actual)))
        if actual == expected));
    check!(matches!(
        grant.charge_steps(0),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(matches!(
        grant.charge_bytes(0),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn archive_physical_summary_is_admitted_before_any_common_reader() -> TestResult {
    let fixture = Fixture::new()?;
    put(
        &fixture.store,
        "row/a",
        envelope("row/a", "null").as_bytes(),
    )?;
    let original = capture_rows(&fixture.store.snapshot())?;
    let wrong_rows = mutated(&original, |archive| {
        let group = archive
            .prefixes
            .first_mut()
            .ok_or("attachment group absent")?;
        group.physical_rows = group
            .physical_rows
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        Ok(())
    })?;
    refused(&wrong_rows, "frozen archive physical summary differs")?;
    let wrong_bytes = mutated(&original, |archive| {
        let group = archive
            .prefixes
            .first_mut()
            .ok_or("attachment group absent")?;
        group.encoded_bytes = group
            .encoded_bytes
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        Ok(())
    })?;
    refused(&wrong_bytes, "frozen archive physical summary differs")
}

#[test]
fn archive_duplicate_and_unordered_physical_rows_cannot_form_an_index() -> TestResult {
    let fixture = Fixture::new()?;
    for key in ["row/a", "row/b"] {
        put(&fixture.store, key, envelope(key, "null").as_bytes())?;
    }
    let original = capture_rows(&fixture.store.snapshot())?;
    let unordered = mutated(&original, |archive| {
        archive
            .prefixes
            .first_mut()
            .ok_or("attachment group absent")?
            .rows
            .reverse();
        Ok(())
    })?;
    refused(&unordered, "frozen archive duplicate or unordered row")?;
    let duplicate = mutated(&original, |archive| {
        let rows = &mut archive
            .prefixes
            .first_mut()
            .ok_or("attachment group absent")?
            .rows;
        let first = rows.first().ok_or("first row absent")?;
        let copy = Row {
            key: first.key.clone(),
            value: first.value,
        };
        rows.insert(0, copy);
        Ok(())
    })?;
    refused(&duplicate, "frozen archive duplicate or unordered row")
}

#[test]
fn archive_outside_row_and_swapped_fixed_groups_refuse_before_source_issuance() -> TestResult {
    let fixture = Fixture::new()?;
    put(
        &fixture.store,
        "row/a",
        envelope("row/a", "null").as_bytes(),
    )?;
    let original = capture_rows(&fixture.store.snapshot())?;
    let outside = mutated(&original, |archive| {
        archive
            .prefixes
            .first_mut()
            .ok_or("attachment group absent")?
            .rows
            .first_mut()
            .ok_or("row absent")?
            .key = Store::journal_key("outside", "row/a");
        Ok(())
    })?;
    refused(&outside, "frozen archive row outside prefix")?;
    let swapped = mutated(&original, |archive| {
        archive.prefixes.reverse();
        Ok(())
    })?;
    refused(&swapped, "frozen archive prefix order")
}

#[test]
fn adopted_reference_does_not_skip_archive_canonical_wire_admission() -> TestResult {
    let fixture = Fixture::new()?;
    let original = capture_rows(&fixture.store.snapshot())?;
    let mut bytes = original.bytes().to_vec();
    bytes.push(b' ');
    let expected = reference(&bytes)?;
    let adopted = Bundle::from_owned_bytes(bytes, expected, &mut AttachmentReplayGrantV1::new())?;
    refused(&adopted, "noncanonical frozen attachment archive")
}
