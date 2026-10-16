use super::*;
use std::cell::Cell;

pub(super) fn verify(store: &Store, sources: &Sources) -> TestResult {
    late_source_failure(store, sources)?;
    wrong_grant(store, sources)?;
    callback_failure(store, sources)
}

fn late_source_failure(store: &Store, sources: &Sources) -> TestResult {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let mut requested = locators();
    let last = requested.get_mut(2).ok_or("last genuine locator")?;
    last.byte_length = last
        .byte_length
        .checked_sub(1)
        .ok_or("late locator length")?;
    let called = Cell::new(false);
    let result = with_attached_html_results(
        &current,
        &sources.result.root().key,
        &requested,
        &mut grant,
        |_, _| {
            called.set(true);
            Ok(())
        },
    );
    check!(
        matches!(result, Err(CrawlError::Store(StoreError::Refused { detail }))
        if detail == "requested row is not an exact successful original row")
    );
    check!(!called.get());
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

fn wrong_grant(store: &Store, sources: &Sources) -> TestResult {
    let snapshot = store.snapshot();
    let mut original = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut original)?;
    let mut replacement = AttachmentReplayGrantV1::new();
    let called = Cell::new(false);
    let result = with_attached_html_results(
        &current,
        &sources.result.root().key,
        &locators(),
        &mut replacement,
        |_, _| {
            called.set(true);
            Ok(())
        },
    );
    check!(matches!(
        result,
        Err(CrawlError::Store(StoreError::EvidenceSubjectAttachment(
            EvidenceSubjectAttachmentStoreError::Domain(EvidenceSubjectAttachmentError::Invalid(
                "checked command grant differs"
            ))
        )))
    ));
    check!(!called.get());
    check!(matches!(
        replacement.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(original.ensure_available().is_ok());
    Ok(())
}

fn callback_failure(store: &Store, sources: &Sources) -> TestResult {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let called = Cell::new(false);
    let result: crate::CrawlResult<Option<()>> = with_attached_html_results(
        &current,
        &sources.result.root().key,
        &locators(),
        &mut grant,
        |_, _| {
            called.set(true);
            Err(StoreError::Refused {
                detail: "controlled attached-row callback refusal".into(),
            }
            .into())
        },
    );
    check!(called.get());
    check!(
        matches!(result, Err(CrawlError::Store(StoreError::Refused { detail }))
        if detail == "controlled attached-row callback refusal")
    );
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}
