use super::*;
use crate::participation_evidence::recorded::read_retained_provider_roster_body_v1;
use census_domain::evidence_subject_attachment::{
    AttachmentReplayGrantV1, EvidenceSubjectAttachmentError,
};

#[test]
fn foreign_body_read_refuses_without_poisoning_the_prepared_review_owner() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let operation = "foreign-body-grant-isolation";
    let mut prepared = prepare(&store, operation, None)?;
    let before = store.snapshot().revision();
    {
        let (evidence, _) = prepared.source_parts()?;
        let mut foreign = AttachmentReplayGrantV1::new();
        check!(read_retained_provider_roster_body_v1(evidence, &mut foreign).is_err());
        check!(matches!(
            foreign.ensure_available(),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    check!(eq; store.snapshot().revision().evidence_generation(), before.evidence_generation());
    check!(eq; store.snapshot().revision().derived_generation(), before.derived_generation());
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let applied = store.apply_roster_source_review_v1(prepared, &approved, &digest)?;
    check!(applied.written());
    let replayed = store.replay_roster_source_review_v1(operation, &digest)?;
    check!(replayed.repeated());
    check!(eq; replayed.receipt(), applied.receipt());
    Ok(())
}
