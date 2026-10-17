use super::*;
use crate::CrawlError;
use census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError;
use census_store::{EvidenceSubjectAttachmentStoreError as Error, StoreError};
use std::cell::Cell;

pub(super) fn verify(store: &Store) -> TestResult {
    absent_group(store)?;
    foreign_grant(store)?;
    callback_failure(store)
}

fn absent_group(store: &Store) -> TestResult {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let plan = discover_current_html_results(&current, &mut grant)?;
    let called = Cell::new(false);
    let result = plan.with_group(plan.group_count(), &mut grant, |_, _| {
        called.set(true);
        Ok(())
    });
    check!(
        matches!(result, Err(CrawlError::Store(StoreError::Refused { detail }))
        if detail == "result group absent")
    );
    check!(!called.get());
    poisoned(&grant)
}

fn foreign_grant(store: &Store) -> TestResult {
    let snapshot = store.snapshot();
    let mut original = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut original)?;
    let plan = discover_current_html_results(&current, &mut original)?;
    let mut replacement = AttachmentReplayGrantV1::new();
    let called = Cell::new(false);
    let result = plan.with_group(0, &mut replacement, |_, _| {
        called.set(true);
        Ok(())
    });
    check!(matches!(
        result,
        Err(CrawlError::Store(StoreError::EvidenceSubjectAttachment(
            Error::Domain(EvidenceSubjectAttachmentError::Invalid(
                "checked command grant differs"
            ))
        )))
    ));
    check!(!called.get());
    poisoned(&replacement)?;
    check!(original.ensure_available().is_ok());
    Ok(())
}

fn callback_failure(store: &Store) -> TestResult {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let plan = discover_current_html_results(&current, &mut grant)?;
    let called = Cell::new(false);
    let result: crate::CrawlResult<()> = plan.with_group(0, &mut grant, |_, _| {
        called.set(true);
        Err(StoreError::Refused {
            detail: "controlled discovery callback refusal".into(),
        }
        .into())
    });
    check!(called.get());
    check!(
        matches!(result, Err(CrawlError::Store(StoreError::Refused { detail }))
        if detail == "controlled discovery callback refusal")
    );
    poisoned(&grant)
}

fn poisoned(grant: &AttachmentReplayGrantV1) -> TestResult {
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}
