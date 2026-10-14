use super::*;

#[test]
fn prepared_recovery_directory_events_charge_the_eof_n_and_n_minus_one() -> StoreResult<()> {
    let mut exact = PreparedRecoveryBudget::admit(usage(1)?, 1)?;
    {
        let mut account = exact.begin_target()?;
        for _ in 0..4_095 {
            account.directory_next()?;
        }
        account.directory_next()?;
        exceeded(account.directory_next(), "directory events");
    }
    assert_eq!(exact.shell.directory.used, 4_096);
    let mut short = PreparedRecoveryBudget::admit(usage(1)?, 1)?;
    short.shell.directory.limit = 4_095;
    {
        let mut account = short.begin_target()?;
        for _ in 0..4_095 {
            account.directory_next()?;
        }
        exceeded(account.directory_next(), "directory events");
    }
    assert_eq!(short.shell.directory.used, 4_095);
    Ok(())
}

#[test]
fn prepared_recovery_traversal_and_inode_credit_continues_across_targets() -> StoreResult<()> {
    let mut budget = PreparedRecoveryBudget::admit(usage(2)?, 2)?;
    budget.shell.directory.limit = 8_191;
    budget.shell.inode.limit = 8_191;
    {
        let mut account = budget.begin_target()?;
        for _ in 0..4_096 {
            account.directory_next()?;
            account.inode_slot()?;
        }
    }
    {
        let mut account = budget.begin_target()?;
        for _ in 0..4_095 {
            account.directory_next()?;
            account.inode_slot()?;
        }
        exceeded(account.directory_next(), "directory events");
        exceeded(account.inode_slot(), "inode slots");
    }
    assert_eq!(budget.shell.directory.used, 8_191);
    assert_eq!(budget.shell.inode.used, 8_191);
    Ok(())
}

#[test]
fn prepared_recovery_reservation_reads_and_eof_are_separate_cumulative_dimensions(
) -> StoreResult<()> {
    let mut budget = PreparedRecoveryBudget::admit(usage(1)?, 1)?;
    {
        let mut account = budget.begin_target()?;
        account.payload(RecoveryPayloadRole::Reservation, 167_935)?;
        account.payload(RecoveryPayloadRole::Reservation, 1)?;
        exceeded(
            account.payload(RecoveryPayloadRole::Reservation, 1),
            "reservation payload",
        );
        for _ in 0..4_096 {
            account.eof_probe(RecoveryPayloadRole::Reservation)?;
        }
        exceeded(
            account.eof_probe(RecoveryPayloadRole::Reservation),
            "reservation EOF probes",
        );
    }
    assert_eq!(budget.shell.reservation_payload.used, 167_936);
    assert_eq!(budget.shell.reservation_eof.used, 4_096);
    assert_eq!(budget.shell.payload.used, 0);
    assert_eq!(budget.shell.recovery_eof.used, 0);
    Ok(())
}

#[test]
fn prepared_recovery_paths_diagnostics_assessments_enforce_named_boundaries() -> StoreResult<()> {
    let mut budget = PreparedRecoveryBudget::admit(usage(1)?, 1)?;
    {
        let mut account = budget.begin_target()?;
        account.path_bytes(4_096)?;
        exceeded(account.path_bytes(4_097), "path bytes");
        account.diagnostic_bytes(8_191)?;
        account.diagnostic_bytes(1)?;
        exceeded(account.diagnostic_bytes(1), "diagnostic bytes");
        account.assessment_bytes(4_096)?;
        exceeded(account.assessment_bytes(4_097), "assessment bytes");
    }
    assert_eq!(budget.shell.path.used, 4_096);
    assert_eq!(budget.shell.diagnostic.used, 8_192);
    assert_eq!(budget.shell.assessment.used, 4_096);
    assert_eq!(budget.reads.recovery_totals().allocations.owned.used, 0);
    Ok(())
}

#[test]
fn prepared_recovery_direct_operations_have_exact_closed_class_counters() -> StoreResult<()> {
    let mut budget = PreparedRecoveryBudget::admit(usage(2)?, 2)?;
    let operations = [
        RecoveryFsOperation::Inspect,
        RecoveryFsOperation::Open,
        RecoveryFsOperation::Create,
        RecoveryFsOperation::Link,
        RecoveryFsOperation::Rename,
        RecoveryFsOperation::Remove,
        RecoveryFsOperation::Sync,
    ];
    for _ in 0..2 {
        let mut account = budget.begin_target()?;
        for operation in operations {
            account.direct_operation(operation)?;
        }
    }
    for operation in operations {
        assert_eq!(budget.shell.direct.count(operation), 2);
    }
    assert_eq!(budget.shell.directory.used, 0);
    Ok(())
}
