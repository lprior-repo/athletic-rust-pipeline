use super::*;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{EvidenceSubjectAttachmentStoreError as AttachmentError, StoreError};

pub(in super::super) fn moved(fixture: &Fixture, objects: &Path) -> TestResult {
    let snapshot = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let retained = snapshot.read_evidence_subject_attachment_references_v1(&mut grant)?;
    let results = retained.records().get(1).ok_or("original results B")?;
    let subject = results
        .command
        .subjects
        .as_slice()
        .first()
        .ok_or("results B subject")?;
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(matches!(
        current.bind_exact_subject(subject, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::TargetMoved
        ))
    ));
    check!(fixture.capture(objects).is_err());
    Ok(())
}

pub(in super::super) fn historical_status_target_moved(store: &Store) -> TestResult {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let retained = snapshot.read_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(eq; retained.records().len(), 3);
    let article = retained.records().first().ok_or("historical article A")?;
    let results = retained.records().get(1).ok_or("revoked results B")?;
    for subject in article.command.subjects.as_slice() {
        let mut grant = AttachmentReplayGrantV1::new();
        let current =
            snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
        check!(matches!(
            current.bind_exact_subject(subject, &mut grant),
            Err(StoreError::EvidenceSubjectAttachment(
                AttachmentError::TargetMoved
            ))
        ));
    }
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    for subject in results.command.subjects.as_slice() {
        check!(current.bind_exact_subject(subject, &mut grant)?.is_none());
    }
    check!(eq; census_crawl::participation_evidence::recorded::discover_current_html_results(&current, &mut grant)?.group_count(), 0);
    Ok(())
}
