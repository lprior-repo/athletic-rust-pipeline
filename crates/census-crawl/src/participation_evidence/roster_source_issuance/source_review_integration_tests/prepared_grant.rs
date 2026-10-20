use super::*;
use crate::participation_evidence::roster_source_issuance::RosterExistingBindingAssessmentV1;
use census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError;
use census_domain::model::TeamId;
use census_store::{EvidenceSubjectAttachmentStoreError, StoreError, Table};

#[test]
fn prepared_review_refuses_exhausted_grant_before_and_during_apply() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let operation = "exhausted-grant-safety";
    let mut prepared = prepare(&store, operation, None)?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let before = store.snapshot().revision();
    {
        let (_, grant) = prepared.source_parts()?;
        check!(grant.charge_bytes(usize::MAX).is_err());
    }
    check!(prepared.proposal_claim().is_err());
    check!(store
        .apply_roster_source_review_v1(prepared, &approved, &digest)
        .is_err());
    check!(store
        .replay_roster_source_review_v1(operation, &digest)
        .is_err());
    check!(eq; store.snapshot().revision().evidence_generation(), before.evidence_generation());
    check!(eq; store.snapshot().revision().derived_generation(), before.derived_generation());
    Ok(())
}

#[test]
fn missing_inspected_program_poisons_prepared_review_without_writes() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let operation = "missing-inspected-program";
    let mut prepared = prepare(&store, operation, None)?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let sequence = store.snapshot().sequence();
    let tables = store.snapshot().tables_digest(&Table::ALL)?;
    let receipts = store.receipt_count()?;
    {
        let (evidence, grant) = prepared.source_parts()?;
        let absent = TeamId::mint("team", &["absent-inspected-program"]);
        let result = RosterExistingBindingAssessmentV1::read_inspected_parents(
            evidence,
            evidence.requested_school(),
            &absent,
            grant,
        );
        check!(matches!(
            result,
            Err(crate::CrawlError::Store(StoreError::Refused { detail }))
                if detail == "inspected roster program is missing"
        ));
        check!(matches!(
            grant.ensure_available(),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    check!(matches!(
        prepared.proposal_claim(),
        Err(StoreError::EvidenceSubjectAttachment(
            EvidenceSubjectAttachmentStoreError::Domain(EvidenceSubjectAttachmentError::Poisoned)
        ))
    ));
    check!(matches!(
        store.apply_roster_source_review_v1(prepared, &approved, &digest),
        Err(StoreError::EvidenceSubjectAttachment(
            EvidenceSubjectAttachmentStoreError::Domain(EvidenceSubjectAttachmentError::Poisoned)
        ))
    ));
    check!(store
        .receipt(&format!("roster_source_issuance_v1:{operation}"))?
        .is_none());
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, tables);
    check!(eq; store.receipt_count()?, receipts);
    Ok(())
}

#[test]
fn exact_inspected_parents_preserve_prepared_review_and_apply() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let mut prepared = prepare(&store, "exact-inspected-parents", None)?;
    let sequence = store.snapshot().sequence();
    let tables = store.snapshot().tables_digest(&Table::ALL)?;
    let receipts = store.receipt_count()?;
    {
        let (evidence, grant) = prepared.source_parts()?;
        let (school, program) = RosterExistingBindingAssessmentV1::read_inspected_parents(
            evidence,
            evidence.requested_school(),
            evidence.requested_program(),
            grant,
        )?;
        check!(eq; &school.id, evidence.requested_school());
        check!(eq; &program.id, evidence.requested_program());
        check!(eq; &program.school, &school.id);
        grant.ensure_available()?;
    }
    prepared.proposal_claim()?;
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, tables);
    check!(eq; store.receipt_count()?, receipts);
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    check!(store
        .apply_roster_source_review_v1(prepared, &approved, &digest)?
        .written());
    Ok(())
}

#[test]
fn foreign_parent_read_poisons_only_supplied_grant_and_owner_can_apply() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let mut prepared = prepare(&store, "foreign-inspected-parent-grant", None)?;
    let sequence = store.snapshot().sequence();
    let tables = store.snapshot().tables_digest(&Table::ALL)?;
    let receipts = store.receipt_count()?;
    {
        let (evidence, owner) = prepared.source_parts()?;
        let mut foreign = AttachmentReplayGrantV1::new();
        let result = RosterExistingBindingAssessmentV1::read_inspected_parents(
            evidence,
            evidence.requested_school(),
            evidence.requested_program(),
            &mut foreign,
        );
        check!(matches!(
            result,
            Err(crate::CrawlError::Store(
                StoreError::EvidenceSubjectAttachment(EvidenceSubjectAttachmentStoreError::Domain(
                    EvidenceSubjectAttachmentError::Invalid("checked command grant differs")
                ))
            ))
        ));
        check!(matches!(
            foreign.ensure_available(),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
        owner.ensure_available()?;
    }
    prepared.proposal_claim()?;
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, tables);
    check!(eq; store.receipt_count()?, receipts);
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    check!(store
        .apply_roster_source_review_v1(prepared, &approved, &digest)?
        .written());
    Ok(())
}
