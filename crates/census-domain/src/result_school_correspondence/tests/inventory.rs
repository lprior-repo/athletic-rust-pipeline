use super::*;

fn revoked() -> TestResult<[ResultSchoolReviewedRecordV1; 2]> {
    let first = record(command("controlled-original")?)?;
    let mut value = first.command.clone();
    value.operation = text("controlled-retirement")?;
    value.expected_inventory = next(&first)?;
    value.action = ResultSchoolActionV1::Revoke {
        prior: ResultSchoolRecordReferenceV1 {
            operation: first.command.operation.clone(),
            digest: first.record_digest.clone(),
        },
    };
    Ok([first, record(value)?])
}

#[test]
fn repeated_complete_replay_is_byte_preserving_and_never_reactivates_revoked_claim() -> TestResult {
    let rows = revoked()?;
    let first = rows.first().ok_or("first")?;
    let last = rows.last().ok_or("last")?;
    let receipts = rows.iter().map(receipt).collect::<Vec<_>>();
    let expected = next(last)?;
    let before = crate::model::serialized_bytes(&rows)?;
    for _ in 0..2 {
        let mut grant = AttachmentReplayGrantV1::new();
        let projection = replay_result_school_records_v1(
            &first.command.store,
            &first.command.expected_inventory,
            &rows,
            &receipts,
            &expected,
            &mut grant,
        )?;
        check!(eq; projection.active_records().count(), 0);
        check!(eq; crate::model::serialized_bytes(projection.records())?, before);
        check!(eq; projection.inventory(), &expected);
    }
    let mut grant = AttachmentReplayGrantV1::new();
    check!(replay_result_school_records_v1(
        &first.command.store,
        &first.command.expected_inventory,
        std::slice::from_ref(first),
        &[receipt(first)],
        &expected,
        &mut grant
    )
    .is_err());
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[test]
fn receipt_order_foreign_store_missing_records_and_changed_inventory_refuse() -> TestResult {
    let rows = revoked()?;
    let first = rows.first().ok_or("first")?;
    let expected = next(rows.last().ok_or("last")?)?;
    for case in 0..5 {
        let mut records = rows.to_vec();
        let mut receipts = rows.iter().map(receipt).collect::<Vec<_>>();
        let mut target = expected.clone();
        let mut store = first.command.store.clone();
        match case {
            0 => receipts.reverse(),
            1 => store = StoreIdentity::parse(&"b".repeat(64))?,
            2 => {
                records.pop();
            }
            3 => target.digest = digest("foreign-inventory")?,
            _ => receipts.first_mut().ok_or("receipt")?.record_digest = digest("foreign-receipt")?,
        }
        let mut grant = AttachmentReplayGrantV1::new();
        check!(replay_result_school_records_v1(
            &store,
            &first.command.expected_inventory,
            &records,
            &receipts,
            &target,
            &mut grant
        )
        .is_err());
        check!(grant.ensure_available().is_err());
    }
    Ok(())
}

#[test]
fn exact_input_and_person_context_are_committed_by_all_model_record_digests() -> TestResult {
    let baseline = record(command("controlled-input-binding")?)?;
    for case in 0..5 {
        let mut changed = baseline.command.clone();
        match case {
            0 => changed.input_token = digest("changed-input")?,
            1 => changed.evidence_generation = 2,
            2 => changed.derived_generation = 1,
            3 => changed.accepted_head.digest = digest("changed-head")?,
            _ => changed.source.person_inventory.digest = digest("changed-person-inventory")?,
        }
        let changed = record(changed)?;
        check!(ne; changed.prepared_digest, baseline.prepared_digest);
        check!(ne; changed.approval_digest, baseline.approval_digest);
        check!(ne; changed.record_digest, baseline.record_digest);
    }
    Ok(())
}

#[test]
fn duplicate_operation_and_wrong_predecessor_never_reactivate_revoked_model_history() -> TestResult
{
    let original = revoked()?;
    let first = original.first().ok_or("first")?;
    let last = original.last().ok_or("last")?;
    for duplicate_operation in [false, true] {
        let mut value = first.command.clone();
        if !duplicate_operation {
            value.operation = text("controlled-wrong-prior")?;
        }
        value.expected_inventory = next(last)?;
        value.action = ResultSchoolActionV1::Accept {
            prior: ResultSchoolPredecessorV1::After(ResultSchoolRecordReferenceV1 {
                operation: first.command.operation.clone(),
                digest: first.record_digest.clone(),
            }),
        };
        let third = record(value)?;
        let expected = next(&third)?;
        let mut rows = original.to_vec();
        rows.push(third);
        let receipts = rows.iter().map(receipt).collect::<Vec<_>>();
        let mut grant = AttachmentReplayGrantV1::new();
        check!(replay_result_school_records_v1(
            &first.command.store,
            &first.command.expected_inventory,
            &rows,
            &receipts,
            &expected,
            &mut grant
        )
        .is_err());
        check!(grant.ensure_available().is_err());
    }
    Ok(())
}
