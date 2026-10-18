use super::*;

#[test]
fn enrolled_inspection_and_empty_current_projection_are_read_only() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let before = retained_state(&store)?;
    let inspection =
        store.inspect_result_school_correspondence_v1(AttachmentReplayGrantV1::new())?;
    check!(eq; inspection.store_identity(), &store.store_identity()?);
    check!(eq; inspection.inventory().records, 0);
    check!(eq; inspection.evidence_generation(), store.evidence_generation());
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let current =
        snapshot.read_current_result_school_correspondences_v1(&attachments, &mut grant)?;
    check!(eq; current.inventory(), inspection.inventory());
    check!(eq; current.snapshot_sequence(), snapshot.sequence());
    check!(eq; current.records().len(), 0);
    check!(eq; current.receipts().len(), 0);
    check!(eq; current.active_records(&mut grant)?.count(), 0);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn enrollment_bridge_refuses_foreign_origin_and_enrollment_only_bytes() -> TestResult {
    let (_directory, store, _writer) = enrolled()?;
    let mut grant = AttachmentReplayGrantV1::new();
    let proof = store.read_result_school_enrollment_proof_v1(&mut grant)?;
    let foreign = census_domain::model::StoreIdentity::parse(&"f".repeat(64))?;
    check!(matches!(
        replay_result_school_enrollment_proof_v1(
            proof.bytes(),
            &foreign,
            &mut AttachmentReplayGrantV1::new()
        ),
        Err(StoreError::ResultSchoolCorrespondence(Error::Corrupt(
            "enrollment proof binding"
        )))
    ));
    let raw = store
        .meta
        .get(super::super::ENROLLMENT)?
        .ok_or("missing original enrollment")?;
    check!(matches!(
        replay_result_school_enrollment_proof_v1(
            &raw,
            &store.store_identity()?,
            &mut AttachmentReplayGrantV1::new()
        ),
        Err(StoreError::ResultSchoolCorrespondence(Error::Corrupt(
            "record decoding"
        )))
    ));
    Ok(())
}
