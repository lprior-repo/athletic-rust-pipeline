use super::*;

#[test]
fn acceptance_revocation_keeps_complete_history_and_no_active_mapping() -> TestResult {
    let first = record(command("controlled-accept")?)?;
    let mut revoke = first.command.clone();
    revoke.operation = text("controlled-revoke")?;
    revoke.expected_inventory = next(&first)?;
    revoke.action = ResultSchoolActionV1::Revoke {
        prior: ResultSchoolRecordReferenceV1 {
            operation: first.command.operation.clone(),
            digest: first.record_digest.clone(),
        },
    };
    let revoked = record(revoke)?;
    let expected = next(&revoked)?;
    let rows = [first.clone(), revoked.clone()];
    let receipts = [receipt(&first), receipt(&revoked)];
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
    check!(eq; projection.records().len(), 2);
    check!(eq; projection.inventory(), &expected);
    Ok(())
}

#[test]
fn competing_schools_survive_instead_of_last_writer_selection() -> TestResult {
    let first = record(command("controlled-school-a")?)?;
    let mut other = first.command.clone();
    other.operation = text("controlled-school-b")?;
    other.expected_inventory = next(&first)?;
    other.source.target.school = SchoolId::mint("sch", &["other-controlled-school"]);
    let second = record(other)?;
    let expected = next(&second)?;
    let rows = [first.clone(), second.clone()];
    let receipts = [receipt(&first), receipt(&second)];
    let mut grant = AttachmentReplayGrantV1::new();
    let projected = replay_result_school_records_v1(
        &first.command.store,
        &first.command.expected_inventory,
        &rows,
        &receipts,
        &expected,
        &mut grant,
    )?;
    check!(eq; projected.active_records().count(), 2);
    Ok(())
}

#[test]
fn second_first_and_changed_immutable_revoke_refuse() -> TestResult {
    let first = record(command("controlled-first")?)?;
    for changed_revoke in [false, true] {
        let mut invalid = first.command.clone();
        invalid.operation = text("controlled-invalid")?;
        invalid.expected_inventory = next(&first)?;
        if changed_revoke {
            invalid.action = ResultSchoolActionV1::Revoke {
                prior: ResultSchoolRecordReferenceV1 {
                    operation: first.command.operation.clone(),
                    digest: first.record_digest.clone(),
                },
            };
            invalid.source.target.day = ProfileDate::parse("2026-05-16")?;
        }
        let second = record(invalid)?;
        let expected = next(&second)?;
        let rows = [first.clone(), second.clone()];
        let receipts = [receipt(&first), receipt(&second)];
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
