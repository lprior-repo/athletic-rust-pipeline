use super::*;
use std::os::unix::process::ExitStatusExt;

#[test]
fn checkpoint_child() -> TestResult {
    let Some(root) = std::env::var_os("GN1J_CRASH_ROOT") else {
        return Ok(());
    };
    let store = Store::open(std::path::Path::new(&root))?;
    match std::env::var("GN1J_CHECKPOINT")?.as_str() {
        "lease" => {
            store.put_input_record(&input()?)?;
            store.input_lease_acquire(&id('a'), 1, "crash-owner", "2999-12-31")?;
        }
        "unit" => {
            completed(&store)?;
        }
        other => return Err(format!("unknown checkpoint {other}").into()),
    }
    std::process::abort()
}
fn crash(checkpoint: &str, root: &std::path::Path) -> TestResult {
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "records_tests::crash::checkpoint_child",
            "--nocapture",
        ])
        .env("GN1J_CRASH_ROOT", root)
        .env("GN1J_CHECKPOINT", checkpoint)
        .output()?;
    check!(eq; output.status.signal(), Some(6));
    Ok(())
}
#[test]
fn atomic_lease_binding_survives_process_abort_and_reclaims_exactly_once() -> TestResult {
    let dir = tempfile::tempdir()?;
    crash("lease", dir.path())?;
    let store = Store::open(dir.path())?;
    let input = store.get_input_record(&id('a'))?.ok_or("missing")?;
    let lease = store.get_lease_record(&id('a'))?.ok_or("missing")?;
    check!(eq; input.revision(), 2);
    check!(eq; input.lease_epoch(), Some(lease.epoch()));
    check!(eq; store.meta.prefix(b"lease/").count(), 1);
    store.lease_release(&id('a'), lease.epoch())?;
    check!(eq; store.meta.prefix(b"lease/").count(), 0);
    check!(matches!(
        store.lease_release(&id('a'), lease.epoch()),
        Err(StoreError::LeaseMismatch { actual: None, .. })
    ));
    let (epoch, bound) = store.input_lease_acquire(&id('a'), 2, "recovery-owner", "2999-12-31")?;
    check!(eq; epoch, 2);
    check!(eq; bound.revision, 3);
    Ok(())
}
#[test]
fn completion_output_and_receipt_survive_process_abort() -> TestResult {
    let dir = tempfile::tempdir()?;
    crash("unit", dir.path())?;
    let store = Store::open(dir.path())?;
    let unit = store.get_unit_record(&id('c'))?.ok_or("missing")?;
    check!(eq; unit.revision(), 4);
    check!(store.unit_replay_complete(&id('c'), &[])?);
    check!(matches!(unit.state(), UnitState::Complete { .. }));
    check!(eq; store.get_unit_output(&id('c'))?, Some(b"durable output".to_vec()));
    Ok(())
}
