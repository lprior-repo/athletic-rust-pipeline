use super::*;
use std::sync::{atomic::AtomicBool, Arc};

use super::super::artifacts::MAX_ARTIFACT;
use super::super::process::tests::wait_until_zombie;

#[test]
fn cleanup_retains_both_actual_exits_when_shared_output_failure_and_full_ledger_coincide(
) -> Result<()> {
    let root = tempfile::tempdir()?;
    let (mut endpoint, mut node) = exited_children(root.path())?;
    let endpoint_pid = endpoint.pid();
    let node_pid = node.pid();
    let ledger = std::fs::OpenOptions::new()
        .write(true)
        .open(root.path().join("processes.jsonl"))?;
    ledger.set_len(MAX_ARTIFACT)?;
    let processes = stop_children(&mut endpoint, &mut node);
    assert_resource_failure(&processes.endpoint)?;
    assert_resource_failure(&processes.node)?;
    write_json(&root.path().join("cleanup.json"), &processes.evidence)?;
    let cleanup: Value = serde_json::from_str(&read_bounded(&root.path().join("cleanup.json"))?)?;
    assert_process(&cleanup["endpoint_term_reap"], endpoint_pid, 7)?;
    assert_process(&cleanup["node_term_reap"], node_pid, 9)?;
    assert!(cleanup["endpoint_error"]
        .as_str()
        .is_some_and(|error| error.contains("resource-limit")));
    assert!(cleanup["node_error"]
        .as_str()
        .is_some_and(|error| error.contains("resource-limit")));
    for (name, pid, code) in [("endpoint", endpoint_pid, 7), ("node", node_pid, 9)] {
        let retained: Value = serde_json::from_str(&read_bounded(
            &root.path().join(format!("{name}.process.json")),
        )?)?;
        assert_process(&retained, pid, code)?;
        assert!(retained["process_ledger_error"]
            .as_str()
            .is_some_and(|error| error.contains("resource-limit")));
    }
    assert_eq!(ledger.metadata()?.len(), MAX_ARTIFACT);
    assert_eq!(read_bounded(&node.log)?, "node-output");
    assert!(std::fs::metadata(&endpoint.log)?.len() <= MAX_ARTIFACT);
    Ok(())
}

#[test]
fn ledger_failure_alone_retains_successful_reap_without_certifying_cleanup() -> Result<()> {
    let root = tempfile::tempdir()?;
    let mut process = child(
        root.path(),
        "full-ledger",
        "exit 0",
        Arc::new(AtomicBool::new(false)),
    )?;
    let pid = process.pid();
    wait_until_zombie(pid)?;
    let ledger = std::fs::OpenOptions::new()
        .write(true)
        .open(root.path().join("processes.jsonl"))?;
    ledger.set_len(MAX_ARTIFACT)?;
    let error = process
        .stop()
        .err()
        .context("full process ledger accepted")?;
    assert!(format!("{error:#}").contains("resource-limit"));
    let retained: Value = serde_json::from_str(&read_bounded(
        &root.path().join("full-ledger.process.json"),
    )?)?;
    assert_process(&retained, pid, 0)?;
    assert_eq!(retained["success"], json!(true));
    assert_eq!(retained["output_error"], Value::Null);
    assert!(retained["process_ledger_error"]
        .as_str()
        .is_some_and(|error| error.contains("resource-limit")));
    let repeated = process
        .stop()
        .err()
        .context("retained ledger failure masked")?;
    assert!(format!("{repeated:#}").contains("resource-limit"));
    assert_eq!(ledger.metadata()?.len(), MAX_ARTIFACT);
    Ok(())
}

fn child(root: &Path, name: &str, script: &str, shared: Arc<AtomicBool>) -> Result<OwnedProcess> {
    OwnedProcess::spawn(
        root,
        name,
        Path::new("/usr/bin/sh"),
        &["-c".into(), script.into()],
        &[],
        shared,
    )
}

fn assert_process(evidence: &Value, pid: u32, code: i32) -> Result<()> {
    assert_eq!(evidence["identity"]["pid"], json!(pid));
    assert_eq!(evidence["pid"], json!(pid));
    assert_eq!(evidence["reaped"], json!(true));
    assert_eq!(evidence["actual_exit_status"]["code"], json!(code));
    assert_eq!(evidence["actual_exit_status"]["signal"], Value::Null);
    assert_eq!(evidence["success"], json!(code == 0));
    assert_eq!(evidence["requested_signal"], Value::Null);
    Ok(())
}

fn exited_children(root: &Path) -> Result<(OwnedProcess, OwnedProcess)> {
    let shared = Arc::new(AtomicBool::new(false));
    let count = MAX_ARTIFACT
        .checked_add(1)
        .context("output size overflow")?;
    let endpoint = child(
        root,
        "endpoint",
        &format!("/usr/bin/head -c {count} /dev/zero; exit 7"),
        Arc::clone(&shared),
    )?;
    let node = child(root, "node", "printf node-output; exit 9", shared)?;
    wait_until_zombie(endpoint.pid())?;
    wait_until_zombie(node.pid())?;
    assert!(endpoint.reaped_status().is_none());
    assert!(node.reaped_status().is_none());
    Ok((endpoint, node))
}

fn assert_resource_failure(result: &Result<Value>) -> Result<()> {
    let error = result
        .as_ref()
        .err()
        .context("output resource failure accepted")?;
    assert!(format!("{error:#}").contains("resource-limit"));
    Ok(())
}

#[test]
fn full_ledger_cannot_erase_requested_term_or_actual_signal_exit() -> Result<()> {
    let root = tempfile::tempdir()?;
    let mut process = OwnedProcess::spawn(
        root.path(),
        "term-full-ledger",
        Path::new("/usr/bin/sleep"),
        &["300".into()],
        &[],
        Arc::new(AtomicBool::new(false)),
    )?;
    process.ensure_running()?;
    let pid = process.pid();
    let ledger = std::fs::OpenOptions::new()
        .write(true)
        .open(root.path().join("processes.jsonl"))?;
    ledger.set_len(MAX_ARTIFACT)?;
    let error = process
        .stop()
        .err()
        .context("full ledger cleanup accepted")?;
    assert!(format!("{error:#}").contains("resource-limit"));
    let retained: Value = serde_json::from_str(&read_bounded(
        &root.path().join("term-full-ledger.process.json"),
    )?)?;
    assert_eq!(retained["identity"]["pid"], json!(pid));
    assert_eq!(retained["requested_signal"], json!(15));
    assert!(retained["term_at"].as_str().is_some());
    assert_eq!(retained["reaped"], json!(true));
    assert_eq!(retained["actual_exit_status"]["code"], Value::Null);
    assert_eq!(retained["actual_exit_status"]["signal"], json!(15));
    assert_eq!(retained["actual_exit_status"]["success"], json!(false));
    assert_eq!(retained["success"], json!(false));
    assert_eq!(ledger.metadata()?.len(), MAX_ARTIFACT);
    Ok(())
}
