use super::{require_manifest, wait};
use crate::{free_port, Endpoint, Instant, TestResult, READY_BUDGET};
use serde_json::json;

#[test]
fn endpoint_readiness_requires_consolidate_run_in_a_valid_manifest() -> TestResult {
    let cases = [
        json!({}),
        json!({"services": {}}),
        json!({"services": [{"name": "Other", "handlers": [{"name": "run"}]}]}),
        json!({"services": [{"name": "Consolidate"}]}),
        json!({"services": [{"name": "Consolidate", "handlers": [{"name": "other"}]}]}),
    ];
    for manifest in cases {
        check!(
            require_manifest(&manifest).is_err(),
            "accepted an invalid manifest: {manifest}"
        );
    }
    require_manifest(&json!({
        "services": [{"name": "Consolidate", "handlers": [{"name": "run"}]}]
    }))?;
    Ok(())
}

#[test]
fn endpoint_readiness_refuses_exited_child_and_retains_actual_stderr() -> TestResult {
    let directory = tempfile::tempdir()?;
    let data = directory.path().join("census");
    std::fs::create_dir(&data)?;
    std::fs::write(data.join("fjall"), b"an obstructed store directory")?;
    let mut endpoint = Endpoint::start(&data, free_port()?)?;
    let pid = endpoint.guard.child.id();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let result = runtime.block_on(wait(&mut endpoint, Instant::now() + READY_BUDGET));
    let reason = match result {
        Err(reason) => reason,
        Ok(()) => return Err("an exited census endpoint was declared ready".into()),
    };
    check!(
        reason.contains(&format!("pid={pid}")),
        "missing child identity: {reason}"
    );
    check!(
        reason.contains("exited before readiness"),
        "missing exit outcome: {reason}"
    );
    let status = endpoint
        .guard
        .child
        .try_wait()?
        .ok_or("child did not exit")?;
    check!(
        !status.success(),
        "obstructed startup unexpectedly succeeded"
    );
    check!(
        reason.contains(&status.to_string()),
        "missing actual exit status: {reason}"
    );
    let captured = std::fs::read_to_string(&endpoint.log_path)?;
    check!(
        captured.contains("census-serve:"),
        "missing actual startup diagnostic: {captured}"
    );
    check!(
        reason.contains(&captured),
        "actual stdout/stderr was not retained: {reason}"
    );
    check!(
        reason.contains(&endpoint.log_path.display().to_string()),
        "missing captured log path: {reason}"
    );
    Ok(())
}
