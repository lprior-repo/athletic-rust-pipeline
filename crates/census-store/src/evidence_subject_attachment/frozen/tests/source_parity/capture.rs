use super::*;
use crate::evidence_subject_attachment::tests::workflow::fixture::{
    apply, prepare, written, Context,
};
use crate::public_identity_tests::support::retained_state;

fn committed_root(label: &str) -> TestResult<(Fixture, Vec<u8>, Vec<u8>)> {
    let fixture = Fixture::new()?;
    let context = Context::synthetic(label)?;
    let inspection = fixture.inspect()?;
    let command = context.command(label, &inspection)?;
    written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command, &context)?,
    )?)?;
    let key = Store::journal_key(PHASE, &crate::evidence_subject_attachment::keys::record(1)?);
    let raw = fixture
        .store
        .snapshot()
        .journal_get(&key)?
        .ok_or("committed physical root absent")?
        .as_ref()
        .to_vec();
    Ok((fixture, key, raw))
}

#[test]
fn live_current_accepts_outer_json_whitespace_but_freeze_refuses_without_rewriting() -> TestResult {
    let (fixture, key, raw) = committed_root("frozen-outer-whitespace")?;
    let mut modified = Vec::new();
    modified.extend_from_slice(b" \n");
    modified.extend_from_slice(&raw);
    modified.extend_from_slice(b"\r\n");
    fixture.store.journal.insert(&key, &modified)?;
    let before = retained_state(&fixture.store)?;
    let view = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(matches!(
        current.freeze_closure(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "frozen physical row has outer whitespace"
        )))
    ));
    check!(matches!(
        grant.charge_steps(0),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(matches!(
        grant.charge_bytes(0),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(eq; retained_state(&fixture.store)?, before);
    let reread = view
        .journal_get(&key)?
        .ok_or("root removed by refused freeze")?;
    check!(eq; reread.as_ref(), modified.as_slice());
    Ok(())
}

#[test]
fn actual_current_freeze_keeps_supported_inner_envelope_whitespace_exactly() -> TestResult {
    let (fixture, key, raw) = committed_root("frozen-inner-whitespace")?;
    check!(eq; raw.first(), Some(&b'{'));
    let mut modified = b"{ \n".to_vec();
    modified.extend_from_slice(raw.get(1..).ok_or("root opening brace absent")?);
    fixture.store.journal.insert(&key, &modified)?;
    let before = retained_state(&fixture.store)?;
    let view = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let frozen = current.freeze_closure(&mut grant)?;
    let source = FrozenSource::read(&frozen, &mut AttachmentReplayGrantV1::new())?;
    let found = source.journal_get(&key)?.ok_or("frozen root absent")?;
    check!(eq; found.as_ref(), modified.as_slice());
    check!(eq; retained_state(&fixture.store)?, before);
    check!(eq; inventory(&view)?, inventory(&source)?);
    Ok(())
}
