use super::*;
use std::cell::Cell;

#[test]
fn accepted_test_records_cannot_change_original_tfrrs_namespace_locator_or_digest() -> TestResult {
    for case in [
        attachment::Case::WrongNamespace,
        attachment::Case::ChangedLocator,
        attachment::Case::ChangedInterpretation,
    ] {
        let (_directory, store, input) = setup::install()?;
        attachment::apply(&store, &input, case, 1)?;
        let before = setup::state(&store, &input.context)?;
        rejected(&store)?;
        check!(eq; setup::state(&store, &input.context)?, before);
    }
    Ok(())
}

fn rejected(store: &Store) -> TestResult {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let called = Cell::new(false);
    match discover_current_html_results(&current, &mut grant) {
        Err(_) => {}
        Ok(plan) => {
            check!(eq; plan.group_count(), 1);
            check!(eq; plan.diagnostic_count(), 0);
            check!(plan
                .with_group(0, &mut grant, |_, _| {
                    called.set(true);
                    Ok(())
                })
                .is_err());
        }
    }
    check!(!called.get());
    poisoned(&grant)
}

pub(super) fn foreign_grant(store: &Store) -> TestResult {
    let snapshot = store.snapshot();
    let mut original = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut original)?;
    let plan = discover_current_html_results(&current, &mut original)?;
    let mut foreign = AttachmentReplayGrantV1::new();
    let called = Cell::new(false);
    check!(plan
        .with_group(0, &mut foreign, |_, _| {
            called.set(true);
            Ok(())
        })
        .is_err());
    check!(!called.get());
    poisoned(&foreign)?;
    original.ensure_available()?;
    Ok(())
}

fn poisoned(grant: &AttachmentReplayGrantV1) -> TestResult {
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn frozen_tfrrs_archive_body_change_and_reference_change_are_sticky_and_read_only() -> TestResult {
    let (_directory, store, input) = setup::install()?;
    attachment::apply(&store, &input, attachment::Case::Original, 2)?;
    let bundle = observe::freeze(&store)?;
    let before = setup::state(&store, &input.context)?;
    let mut changed = bundle.bytes().to_vec();
    let first = changed.first_mut().ok_or("nonempty frozen archive")?;
    *first ^= 1;
    let mut body_grant = AttachmentReplayGrantV1::new();
    check!(FrozenEvidenceSubjectAttachmentBundleV1::from_owned_bytes(
        changed,
        bundle.reference().clone(),
        &mut body_grant,
    )
    .is_err());
    poisoned(&body_grant)?;
    let mut wrong_reference = bundle.reference().clone();
    wrong_reference.digest = objects::reference(b"controlled-foreign-archive-digest")?.digest;
    let mut digest_grant = AttachmentReplayGrantV1::new();
    check!(FrozenEvidenceSubjectAttachmentBundleV1::from_owned_bytes(
        bundle.bytes().to_vec(),
        wrong_reference,
        &mut digest_grant,
    )
    .is_err());
    poisoned(&digest_grant)?;
    check!(eq; setup::state(&store, &input.context)?, before);
    Ok(())
}
