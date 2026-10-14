use super::*;

#[test]
fn prepared_recovery_phase_payload_and_eof_are_distinct_once_only_caps() -> StoreResult<()> {
    let mut budget = PreparedRecoveryBudget::admit(usage(1)?, 1)?;
    {
        let mut account = budget.begin_target()?;
        account.payload(RecoveryPayloadRole::Body, 100_663_296)?;
        account.payload(RecoveryPayloadRole::Metadata, 196_608)?;
        for _ in 0..6 {
            account.eof_probe(RecoveryPayloadRole::Body)?;
        }
        exceeded(
            account.payload(RecoveryPayloadRole::Body, 1),
            "backing bytes",
        );
        exceeded(
            account.eof_probe(RecoveryPayloadRole::Metadata),
            "recovery EOF probes",
        );
        account.start_publication()?;
        refused(account.start_publication());
        refused(account.reads().row(1, 1));
        account.payload(RecoveryPayloadRole::Body, 67_108_864)?;
        account.payload(RecoveryPayloadRole::Metadata, 131_072)?;
        for _ in 0..4 {
            account.eof_probe(RecoveryPayloadRole::Body)?;
        }
        exceeded(
            account.eof_probe(RecoveryPayloadRole::Metadata),
            "publication EOF probes",
        );
        exceeded(
            account.payload(RecoveryPayloadRole::Metadata, 1),
            "backing bytes",
        );
    }
    assert_eq!(budget.shell.payload.used, limits::PAYLOAD);
    assert_eq!(budget.shell.recovery_eof.used, 6);
    assert_eq!(budget.shell.publication_eof.used, 4);
    Ok(())
}

#[test]
fn prepared_recovery_publication_headroom_n_minus_one_has_no_eligible_effect_credit(
) -> StoreResult<()> {
    let mut budget = PreparedRecoveryBudget::admit(usage(1)?, 1)?;
    budget.shell.publication_eof.limit = 3;
    {
        let mut account = budget.begin_target()?;
        exceeded(account.start_publication(), "publication EOF probes");
        refused(account.direct_operation(RecoveryFsOperation::Open));
        refused(account.directory_next());
        refused(account.start_publication());
    }
    assert_eq!(budget.shell.publication_payload.used, 0);
    assert_eq!(budget.shell.publication_eof.used, 0);
    assert_eq!(budget.shell.direct.count(RecoveryFsOperation::Open), 0);
    Ok(())
}

#[test]
fn prepared_recovery_staged_key_probes_do_not_get_a_fourth_receipt_or_third_key() -> StoreResult<()>
{
    let mut budget = PreparedRecoveryBudget::admit(usage(1)?, 1)?;
    let mut account = budget.begin_target()?;
    account.staged_key_probe()?;
    account.staged_key_probe()?;
    exceeded(account.staged_key_probe(), "staged key probes");
    for _ in 0..3 {
        account.reads().receipt_probe()?;
    }
    exceeded(account.reads().receipt_probe(), "receipt probes");
    account.start_publication()?;
    refused(account.staged_key_probe());
    Ok(())
}

#[test]
fn prepared_recovery_io_borrow_debits_shared_account_and_uses_shared_scratch() -> StoreResult<()> {
    let mut budget = PreparedRecoveryBudget::admit(usage(1)?, 1)?;
    {
        let mut account = budget.begin_target()?;
        account.reserve_scratch()?;
        let mut scratch = [0_u8; 32_768];
        {
            let mut io = super::super::io::RecoveryArchiveIo::new(&mut account, &mut scratch);
            io.payload(RecoveryPayloadRole::Metadata, 1)?;
            io.eof_probe(RecoveryPayloadRole::Metadata)?;
            io.reserve_owned(32)?;
            io.path_bytes(32)?;
            io.inode_slot()?;
            io.directory_next()?;
            io.direct_operation(RecoveryFsOperation::Inspect)?;
            io.scratch().fill(7);
            io.release_owned(32)?;
        }
        assert_eq!(scratch.first(), Some(&7));
        assert_eq!(scratch.last(), Some(&7));
    }
    assert_eq!(budget.shell.payload.used, 1);
    assert_eq!(budget.shell.recovery_eof.used, 1);
    assert_eq!(budget.shell.directory.used, 1);
    assert_eq!(budget.shell.inode.used, 1);
    assert_eq!(budget.shell.path.used, 32);
    assert_eq!(budget.shell.direct.count(RecoveryFsOperation::Inspect), 1);
    assert_eq!(budget.reads.recovery_totals().allocations.owned.used, 32);
    assert_eq!(
        budget.reads.recovery_totals().allocations.scratch.used,
        limits::SCRATCH
    );
    Ok(())
}

#[test]
fn prepared_recovery_owner_gate_consumes_first_target_without_begin_reset() -> StoreResult<()> {
    let mut budget = PreparedRecoveryBudget::admit(usage(2)?, 2)?;
    budget.prepare_owner()?;
    budget.reads().admission_value(12, 65_536)?;
    budget.reads().reserve_recovery_owned(4_096)?;
    assert_eq!(budget.targets.used, 1);
    assert_eq!(budget.reads.recovery_totals().admission.used, 65_548);
    assert_eq!(budget.reads.recovery_totals().allocations.owned.used, 4_096);
    refused(budget.prepare_owner());
    refused(budget.begin_target());
    assert_eq!(budget.targets.used, 1);
    assert_eq!(budget.reads.recovery_totals().allocations.owned.used, 4_096);
    Ok(())
}

#[test]
fn prepared_recovery_historical_work_allowances_keep_old_behavior() -> StoreResult<()> {
    let mut budget = crate::acquisition::AcquisitionReadBudget::for_work();
    for _ in 0..84 {
        budget.row(1, 1)?;
    }
    budget.backing(33_619_968)?;
    budget.begin_operation();
    exceeded(budget.row(1, 1), "roots");
    exceeded(budget.backing(1), "backing bytes");
    budget.reserve_recovery_owned(usize::MAX)?;
    assert_eq!(budget.selection_usage().roots(), 84);
    assert_eq!(budget.selection_usage().encoded_bytes(), 168);
    Ok(())
}
