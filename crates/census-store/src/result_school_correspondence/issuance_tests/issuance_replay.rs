use super::*;

#[test]
fn empty_enrolled_closure_replays_after_store_drop_with_exact_binding() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    let before = retained_state(&store)?;
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let current =
        snapshot.read_current_result_school_correspondences_v1(&attachments, &mut grant)?;
    let identity = current.store_identity().clone();
    let sequence = current.snapshot_sequence();
    let inventory = current.inventory().clone();
    let frozen_identity = snapshot.freeze_public_identity_v2()?;
    let proof = current.freeze_closure(&frozen_identity, &mut grant)?;
    check!(eq; retained_state(&store)?, before);
    drop(current);
    drop(attachments);
    drop(snapshot);
    drop(writer);
    drop(store);
    let mut grant = AttachmentReplayGrantV1::new();
    let replayed = replay_result_school_source_proof_v1(
        proof.bytes(),
        proof.reference(),
        &identity,
        sequence,
        &mut grant,
    )?;
    check!(eq; replayed.store_identity(), &identity);
    check!(eq; replayed.snapshot_sequence(), sequence);
    check!(eq; replayed.inventory(), &inventory);
    check!(eq; replayed.records().len(), 0);
    check!(eq; replayed.receipts().len(), 0);
    check!(eq; replayed.active_records(&mut grant)?.count(), 0);
    Ok(())
}

#[test]
fn source_closure_digest_foreign_origin_and_snapshot_fail_closed() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let (_other_directory, other, _other_writer) =
        crate::public_identity_tests::support::fixture()?;
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let current =
        snapshot.read_current_result_school_correspondences_v1(&attachments, &mut grant)?;
    let frozen_identity = snapshot.freeze_public_identity_v2()?;
    let proof = current.freeze_closure(&frozen_identity, &mut grant)?;
    let sequence = snapshot.sequence();
    let identity = store.store_identity()?;
    let mut changed = proof.bytes().to_vec();
    changed.push(b' ');
    let mut reference = proof.reference().clone();
    reference.bytes = u64::try_from(changed.len())?;
    check!(matches!(
        replay_result_school_source_proof_v1(
            &changed,
            &reference,
            &identity,
            sequence,
            &mut AttachmentReplayGrantV1::new()
        ),
        Err(StoreError::ResultSchoolCorrespondence(Error::Corrupt(
            "source closure digest"
        )))
    ));
    check!(matches!(
        replay_result_school_source_proof_v1(
            proof.bytes(),
            proof.reference(),
            &other.store_identity()?,
            sequence,
            &mut AttachmentReplayGrantV1::new()
        ),
        Err(StoreError::ResultSchoolCorrespondence(Error::Corrupt(
            "source closure origin/snapshot/schema"
        )))
    ));
    check!(matches!(
        replay_result_school_source_proof_v1(
            proof.bytes(),
            proof.reference(),
            &identity,
            sequence.checked_add(1).ok_or("test sequence exhausted")?,
            &mut AttachmentReplayGrantV1::new()
        ),
        Err(StoreError::ResultSchoolCorrespondence(Error::Corrupt(
            "source closure origin/snapshot/schema"
        )))
    ));
    Ok(())
}
