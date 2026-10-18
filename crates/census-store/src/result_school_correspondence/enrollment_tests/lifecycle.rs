use super::*;

#[test]
fn seven_and_eight_enroll_preserve_every_original_row_and_cold_replay() -> TestResult {
    for roster in [false, true] {
        let (directory, store, writer) = fixture()?;
        if roster {
            predecessor::enroll_roster(&store)?;
        }
        let before = retained_state(&store)?;
        let inputs = store.inspect_result_school_enrollment_v1(AttachmentReplayGrantV1::new())?;
        check!(eq; inputs.store_identity(), &store.store_identity()?);
        check!(eq; inputs.accepted_head().sequence, 1);
        check!(eq; inputs.prior_capability().lifecycle(), if roster { 8 } else { 7 });
        let prepared = store.prepare_result_school_enrollment_v1(inputs, review()?)?;
        let approval = prepared.approval_bytes().to_vec();
        let digest = prepared.approval_digest().clone();
        check!(eq; retained_state(&store)?, before);
        let first = store.apply_result_school_enrollment_v1(prepared, &approval, &digest)?;
        check!(first.written());
        check!(eq; store.store_format()?.lifecycle_revision, Some(9));
        let after = retained_state(&store)?;
        preserve_original(&before, &after)?;
        let snapshot = store.snapshot();
        let mut grant = AttachmentReplayGrantV1::new();
        let proof = snapshot
            .result_school_enrollment_proof_v1(&mut grant)?
            .ok_or("missing enrollment proof")?;
        check!(eq; replay_result_school_enrollment_proof_v1(proof.bytes(), &store.store_identity()?, &mut grant)?.lifecycle(), if roster { 8 } else { 7 });
        let retained = proof.bytes().to_vec();
        drop(snapshot);
        let replay = store.prepare_result_school_enrollment_replay_v1(
            &review()?.operation,
            &digest,
            AttachmentReplayGrantV1::new(),
        )?;
        let repeated = store.apply_result_school_enrollment_v1(replay, &approval, &digest)?;
        check!(matches!(repeated, Application::Repeated(_)));
        check!(eq; repeated.receipt(), first.receipt());
        check!(eq; retained_state(&store)?, after);
        store.flush()?;
        drop(writer);
        drop(store);
        let (store, _writer) =
            Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
                .into_parts();
        check!(eq; retained_state(&store)?, after);
        check!(eq; store.read_result_school_enrollment_proof_v1(&mut AttachmentReplayGrantV1::new())?.bytes(), retained.as_slice());
        let replay = store.prepare_result_school_enrollment_replay_v1(
            &review()?.operation,
            &digest,
            AttachmentReplayGrantV1::new(),
        )?;
        check!(eq; store.apply_result_school_enrollment_v1(replay, &approval, &digest)?.receipt(), first.receipt());
        check!(eq; retained_state(&store)?, after);
    }
    Ok(())
}

fn preserve_original(
    before: &crate::public_identity_tests::support::PhysicalState,
    after: &crate::public_identity_tests::support::PhysicalState,
) -> TestResult {
    for (table, rows) in before {
        let (_, current) = after
            .iter()
            .find(|(name, _)| name == table)
            .ok_or("missing keyspace")?;
        for (key, bytes) in rows {
            if table == crate::META
                && key.as_slice() == crate::format::LIFECYCLE_REVISION_KEY.as_bytes()
            {
                continue;
            }
            check!(current
                .iter()
                .any(|(held_key, held)| held_key == key && held == bytes));
        }
    }
    Ok(())
}
