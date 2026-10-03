use super::*;
use anyhow::Context;
use std::process::Command;

struct FailureFixture {
    root: tempfile::TempDir,
    ssh: Ssh,
    vm: Process,
}

impl FailureFixture {
    fn start() -> Result<Self> {
        let root = tempfile::tempdir()?;
        let ssh = Ssh {
            root: root.path().into(),
            port: 0,
        };
        let mut vm = Process::spawn(
            root.path(),
            "qemu",
            Command::new("/bin/sh").args(["-c", "exit 1"]),
        )?;
        assert_eq!(vm.wait(100)?.code(), Some(1));
        Ok(Self { root, ssh, vm })
    }

    fn capture(&mut self) -> Result<anyhow::Error> {
        let primary = std::fs::File::open(self.root.path().join("missing-readiness-input"))
            .err()
            .context("missing input unexpectedly exists")?;
        Ok(retain_failure(
            &self.ssh,
            self.root.path(),
            &json!({"clock":{"boot_id":"before"}}),
            &json!({"boot_id":"after"}),
            &mut self.vm,
            anyhow::Error::new(primary),
        ))
    }
}

#[test]
fn diagnostic_failures_publish_owner_evidence_without_replacing_primary_error() -> Result<()> {
    let mut fixture = FailureFixture::start()?;
    let error = fixture.capture()?;
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .map(std::io::Error::kind),
        Some(std::io::ErrorKind::NotFound)
    );
    let evidence = artifacts::json(
        &fixture
            .root
            .path()
            .join("recovered-supervisor-failure.json"),
    )?;
    assert_eq!(evidence.pointer("/qemu_owner/reaped"), Some(&json!(true)));
    assert_eq!(evidence.pointer("/qemu_owner/exit/code"), Some(&json!(1)));
    assert_eq!(
        evidence.pointer("/qemu_owner/requested_signal"),
        Some(&Value::Null)
    );
    let failures = ["/systemd/failure", "/journal/failure"]
        .into_iter()
        .map(|path| {
            evidence
                .pointer(path)
                .and_then(Value::as_str)
                .with_context(|| format!("independent diagnostic failure absent: {path}"))
        })
        .collect::<Result<Vec<_>>>()?;
    let message = format!("{error:#}");
    failures
        .iter()
        .for_each(|failure| assert!(message.contains(failure)));
    Ok(())
}

#[test]
fn publication_failure_preserves_primary_and_both_diagnostic_failures() -> Result<()> {
    let mut fixture = FailureFixture::start()?;
    let pending = fixture
        .root
        .path()
        .join("recovered-supervisor-failure.pending");
    artifacts::write(&pending, b"retained evidence")?;
    let error = fixture.capture()?;
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .map(std::io::Error::kind),
        Some(std::io::ErrorKind::NotFound)
    );
    assert_eq!(artifacts::read(&pending)?, b"retained evidence");
    assert!(!fixture
        .root
        .path()
        .join("recovered-supervisor-failure.json")
        .exists());
    let message = format!("{error:#}");
    assert!(message.contains("recovered supervisor failure evidence publication failed"));
    assert!(message.contains("recovered-supervisor-systemd-show failed"));
    assert!(message.contains("recovered-supervisor-journal failed"));
    assert_eq!(fixture.vm.evidence().pointer("/exit/code"), Some(&json!(1)));
    Ok(())
}
