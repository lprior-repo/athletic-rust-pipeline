use super::*;
use crate::evidence_subject_attachment::{
    RetainedCaptureRootSchemaV1 as Schema, RetainedCaptureRootV1,
};

const KEY: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const SCHEMA: Schema = Schema::HtmlResultCaptureV1;

fn descriptor_root() -> TestResult<RetainedCaptureRootV1> {
    Ok(RetainedCaptureRootV1 {
        schema: SCHEMA,
        key: KEY.to_owned(),
        payload: reference(b"null")?,
    })
}

#[test]
fn exact_external_root_and_canonical_absent_chunk_are_covered_but_text_alias_is_not() -> TestResult
{
    let fixture = Fixture::new()?;
    let physical = Store::journal_key(SCHEMA.phase(), KEY);
    let raw = envelope(KEY, "null");
    fixture.store.journal.insert(&physical, raw.as_bytes())?;
    let view = fixture.store.snapshot();
    let bundle = capture_roots(&view, &[descriptor_root()?])?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    check!(eq; frozen.journal_size(&physical)?, Some(u32::try_from(raw.len())?));
    let value = frozen
        .journal_get(&physical)?
        .ok_or("captured root absent")?;
    check!(eq; value.as_ref(), raw.as_bytes());
    let absent_chunk = chunk_key(&physical, 0)?;
    check!(eq; frozen.journal_size(&absent_chunk)?, None);
    check!(frozen.journal_get(&absent_chunk)?.is_none());
    for separator in [0_u8, 2] {
        let mut noncanonical = physical.clone();
        noncanonical.push(separator);
        noncanonical.extend_from_slice(&[0; 4]);
        outside(frozen.journal_get(&noncanonical))?;
    }
    let alias = Store::journal_key(SCHEMA.phase(), &format!("{KEY}/alias"));
    outside(frozen.journal_size(&alias))?;
    outside(frozen.journal_get(&alias))?;
    let other = Store::journal_key(SCHEMA.phase(), &"0".repeat(64));
    outside(frozen.journal_get(&other))?;
    outside(frozen.scan_journal_namespace(
        JournalNamespace::whole_phase(SCHEMA.phase())?,
        budget(2),
        &mut (),
        |_| Ok(()),
        |_, _, _| Ok(()),
    ))?;
    Ok(())
}

#[test]
fn external_complete_scan_keeps_foreign_prefix_rows_for_common_semantic_refusal() -> TestResult {
    let fixture = Fixture::new()?;
    let physical = Store::journal_key(SCHEMA.phase(), KEY);
    fixture
        .store
        .journal
        .insert(&physical, manifest_value(KEY, 1, 4)?)?;
    let alias = format!("{KEY}/foreign");
    fixture.store.journal.insert(
        Store::journal_key(SCHEMA.phase(), &alias),
        envelope(&alias, "null").as_bytes(),
    )?;
    let view = fixture.store.snapshot();
    let bundle = capture_roots(&view, &[descriptor_root()?])?;
    let frozen = FrozenSource::read(&bundle, &mut AttachmentReplayGrantV1::new())?;
    foreign_completion(&view)?;
    foreign_completion(&frozen)?;
    outside(frozen.journal_get(&Store::journal_key(SCHEMA.phase(), &alias)))?;
    Ok(())
}

fn foreign_completion(source: &impl RetainedJournalReadSource) -> TestResult {
    let mut seen = Vec::new();
    let result = scan_roots(
        source,
        JournalNamespace::new(SCHEMA.phase(), KEY)?,
        budget(2),
        &mut seen,
        |_| Ok(()),
        |seen, key| {
            seen.push(key.to_owned());
            Ok(())
        },
    );
    check!(
        matches!(result, Err(StoreError::JournalDescriptor { phase, key, detail })
        if phase == SCHEMA.phase() && key == KEY && detail == "logical root has missing physical chunks")
    );
    check!(eq; seen, vec![KEY.to_owned()]);
    Ok(())
}
