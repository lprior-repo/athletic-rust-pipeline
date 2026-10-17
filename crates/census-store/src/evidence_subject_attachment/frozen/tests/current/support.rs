use super::*;
use sha2::{Digest, Sha256};

pub(super) fn attach(fixture: &Fixture, label: &str) -> TestResult<(Context, Receipt)> {
    let context = Context::synthetic(label)?;
    let inspection = fixture.inspect()?;
    let command = context.command(label, &inspection)?;
    let receipt = written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command, &context)?,
    )?)?;
    Ok((context, receipt))
}

pub(super) fn committed(label: &str) -> TestResult<(Fixture, Context, Receipt)> {
    let fixture = Fixture::new()?;
    let (context, receipt) = attach(&fixture, label)?;
    Ok((fixture, context, receipt))
}

pub(super) fn capture(store: &Store) -> TestResult<(Bundle, AcceptedPublicIdentitySnapshot)> {
    let view = store.snapshot();
    let accepted = view
        .accepted_public_identity_snapshot()?
        .ok_or("accepted held owner absent")?;
    let mut grant = AttachmentReplayGrantV1::new();
    let current = view.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted, &mut grant,
    )?;
    let bundle = current.freeze_closure(&mut grant)?;
    drop(current);
    Ok((bundle, accepted))
}

pub(super) fn poisoned(grant: &mut AttachmentReplayGrantV1) -> TestResult {
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

pub(super) fn reference(bytes: &[u8]) -> TestResult<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(bytes)))?,
        bytes: u64::try_from(bytes.len())?,
    })
}

pub(super) fn adopt(bytes: Vec<u8>) -> TestResult<Bundle> {
    let expected = reference(&bytes)?;
    Ok(Bundle::from_owned_bytes(
        bytes,
        expected,
        &mut AttachmentReplayGrantV1::new(),
    )?)
}
