use super::*;
use std::process::Command;

mod fixture;

#[test]
fn expected_signal_between_reason_and_either_probe_preserves_real_drain_outcome() -> Result<()> {
    [15, 2].into_iter().try_for_each(|signal| -> Result<()> {
        ["endpoint", "node"]
            .into_iter()
            .try_for_each(|probe| -> Result<()> {
                ["none", "endpoint", "node"]
                    .into_iter()
                    .try_for_each(|failure| run_isolated(signal, probe, failure))
            })
    })
}

#[test]
fn expected_signal_does_not_mask_independent_capture_resource_failure() -> Result<()> {
    run_isolated(15, "endpoint", "resource")
}

fn run_isolated(signal: i32, probe: &str, failure: &str) -> Result<()> {
    let root = tempfile::tempdir()?;
    let mut command = Command::new(std::env::current_exe()?);
    command.args([
        "--exact",
        "qualification_native_vm::guest::supervision::tests::fixture::shutdown_schedule_fixture",
        "--nocapture",
    ]);
    command.env("QUALIFICATION_SHUTDOWN_ROOT", root.path());
    command.env("QUALIFICATION_SHUTDOWN_SIGNAL", signal.to_string());
    command.env("QUALIFICATION_SHUTDOWN_PROBE", probe);
    command.env("QUALIFICATION_SHUTDOWN_FAILURE", failure);
    let mut child = Process::spawn(root.path(), "shutdown-fixture", &mut command)?;
    assert_eq!(child.wait(300)?.code(), Some(0));
    let report = artifacts::json(&root.path().join("schedule.json"))?;
    assert_eq!(report.get("signal"), Some(&json!(signal)));
    assert_eq!(
        report.get("initial_reason"),
        Some(&json!(if failure == "resource" { signal } else { 0 }))
    );
    let certificate = artifacts::json(&root.path().join("current-drain.json"))?;
    verify_certificate(root.path(), &certificate, failure)
}

fn verify_certificate(root: &Path, certificate: &Value, failure: &str) -> Result<()> {
    assert_eq!(certificate.get("orderly"), Some(&json!(failure == "none")));
    assert_eq!(
        certificate.pointer("/endpoint_exit/reaped"),
        Some(&json!(true))
    );
    assert_eq!(certificate.pointer("/node_exit/reaped"), Some(&json!(true)));
    if failure == "none" {
        verify_success(root, certificate)?;
    } else {
        assert!(drain::verify(root, "isolated-boot").is_err());
        if failure != "resource" {
            assert_eq!(
                certificate.pointer(&format!("/{failure}_exit/exit/code")),
                Some(&json!(7))
            );
        } else {
            let log = certificate
                .pointer("/endpoint_exit/identity/log")
                .and_then(Value::as_str)
                .context("resource endpoint log absent")?;
            let bytes = artifacts::read(Path::new(log))?;
            assert!(u64::try_from(bytes.len())? <= artifacts::LIMIT);
            assert!(bytes
                .windows(b"RESOURCE_LIMIT".len())
                .any(|part| part == b"RESOURCE_LIMIT"));
        }
    }
    Ok(())
}

fn verify_success(root: &Path, certificate: &Value) -> Result<()> {
    assert_eq!(&drain::verify(root, "isolated-boot")?, certificate);
    assert_eq!(
        certificate.pointer("/endpoint_exit/exit/code"),
        Some(&json!(0))
    );
    assert_eq!(certificate.pointer("/node_exit/exit/code"), Some(&json!(0)));
    assert_eq!(
        certificate.get("counts"),
        Some(
            &json!({"accepted":3,"completed":2,"cancelled":1,"timed_out":0,"aborted":0,"panicked":0})
        )
    );
    Ok(())
}
