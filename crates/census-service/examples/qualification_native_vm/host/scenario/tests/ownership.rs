use super::*;
use crate::qualification_native_vm::host::{cleanup, machine, Host, Seed};
use std::net::{Ipv4Addr, TcpListener};
use std::process::Command;

struct ExitedVm {
    root: tempfile::TempDir,
    ssh: Ssh,
    vm: Process,
    _reservation: TcpListener,
}

impl ExitedVm {
    fn start() -> Result<Self> {
        let root = tempfile::tempdir()?;
        let reservation = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let ssh = Ssh {
            root: root.path().into(),
            port: reservation.local_addr()?.port(),
        };
        let mut vm = Process::spawn(
            root.path(),
            "qemu",
            Command::new("/bin/sh").args(["-c", "exit 1"]),
        )?;
        assert_eq!(vm.wait(100)?.code(), Some(1));
        Ok(Self {
            root,
            ssh,
            vm,
            _reservation: reservation,
        })
    }
}

#[test]
fn exited_qemu_refuses_boot_when_ssh_cannot_supply_a_boot_id() -> Result<()> {
    let mut fixture = ExitedVm::start()?;
    let error = fixture
        .ssh
        .wait_boot(None, &mut fixture.vm)
        .err()
        .context("exited VM boot was accepted")?;
    assert!(format!("{error:#}").contains("exited"), "{error:#}");
    let evidence = fixture.vm.evidence();
    assert_eq!(evidence.pointer("/exit/code"), Some(&json!(1)));
    assert_eq!(evidence.get("reaped"), Some(&json!(true)));
    assert_eq!(evidence.get("requested_signal"), Some(&Value::Null));
    let bytes = artifacts::read(&fixture.root.path().join("processes.jsonl"))?;
    let events = std::str::from_utf8(&bytes)?
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<serde_json::Result<Vec<_>>>()?;
    let spawned = events
        .iter()
        .filter(|event| event.get("event") == Some(&json!("spawn")))
        .map(|event| event.pointer("/identity/label"))
        .collect::<Vec<_>>();
    assert_eq!(spawned, vec![Some(&json!("qemu"))]);
    Ok(())
}

#[test]
fn dead_qemu_cleanup_retains_primary_failure_and_never_certifies_guest_drain() -> Result<()> {
    let mut fixture = ExitedVm::start()?;
    let mut seed = Seed::start(fixture.root.path(), "ssh-ed25519 test")?;
    let primary = fixture
        .ssh
        .wait_boot(None, &mut fixture.vm)
        .err()
        .context("exited VM boot was accepted")?;
    let original = primary.root_cause().to_string();
    let error = cleanup::finish(
        fixture.root.path(),
        &fixture.ssh,
        &mut seed,
        &mut fixture.vm,
        Err(primary),
    )
    .err()
    .context("scenario and failed cleanup were accepted")?;
    assert_eq!(error.root_cause().to_string(), original);
    let message = format!("{error:#}");
    assert!(message.contains("guest drain unproven failed"), "{message}");
    assert!(message.contains("QEMU cleanup failed"), "{message}");
    let cleanup = artifacts::json(&fixture.root.path().join("cleanup.json"))?;
    assert_eq!(cleanup.get("qemu_orderly"), Some(&json!(false)));
    assert_eq!(cleanup.get("qemu_state"), Some(&json!("UNPROVEN")));
    assert_eq!(cleanup.get("guest_drain_certificate"), Some(&Value::Null));
    assert_eq!(cleanup.pointer("/qemu_exit/exit/code"), Some(&json!(1)));
    assert_eq!(cleanup.pointer("/qemu_exit/reaped"), Some(&json!(true)));
    assert_eq!(
        cleanup.pointer("/qemu_exit/requested_signal"),
        Some(&Value::Null)
    );
    assert!(!fixture.root.path().join("current-drain.json").exists());
    Ok(())
}

#[test]
fn failed_evidence_publication_keeps_scenario_and_all_cleanup_failures_visible() -> Result<()> {
    let mut fixture = ExitedVm::start()?;
    let mut seed = Seed::start(fixture.root.path(), "ssh-ed25519 test")?;
    artifacts::write(&fixture.root.path().join("cleanup.pending"), b"retained")?;
    let primary = fixture
        .ssh
        .wait_boot(None, &mut fixture.vm)
        .err()
        .context("exited VM boot was accepted")?;
    let original = primary.root_cause().to_string();
    let error = cleanup::finish(
        fixture.root.path(),
        &fixture.ssh,
        &mut seed,
        &mut fixture.vm,
        Err(primary),
    )
    .err()
    .context("failed cleanup publication was accepted")?;
    assert_eq!(error.root_cause().to_string(), original);
    let message = format!("{error:#}");
    assert!(message.contains("guest drain unproven failed"), "{message}");
    assert!(message.contains("QEMU cleanup failed"), "{message}");
    assert!(
        message.contains("cleanup evidence publication failed"),
        "{message}"
    );
    assert_eq!(
        artifacts::read(&fixture.root.path().join("cleanup.pending"))?,
        b"retained"
    );
    assert_eq!(fixture.vm.evidence().pointer("/exit/code"), Some(&json!(1)));
    Ok(())
}

#[test]
fn linux_socket_preflight_accepts_107_bytes_and_rejects_108_bytes() -> Result<()> {
    let maximum_root = std::path::PathBuf::from(format!("/{}", "a".repeat(97)));
    assert_eq!(
        machine::qmp_socket(&maximum_root)?,
        maximum_root.join("qmp.sock")
    );
    let invalid_root = std::path::PathBuf::from(format!("/{}", "a".repeat(98)));
    let error = machine::qmp_socket(&invalid_root)
        .err()
        .context("108-byte Linux UNIX pathname was accepted")?;
    assert!(error.to_string().contains("108 bytes"), "{error:#}");
    let utf8_root = std::path::PathBuf::from(format!("/{}", "é".repeat(49)));
    let error = machine::qmp_socket(&utf8_root)
        .err()
        .context("108-byte multibyte UNIX pathname was accepted")?;
    assert!(error.to_string().contains("108 bytes"), "{error:#}");
    Ok(())
}

#[test]
fn invalid_socket_root_fails_before_payload_keys_disks_or_children_are_prepared() -> Result<()> {
    let retained = tempfile::tempdir()?;
    let root = retained.path().join("a".repeat(108));
    std::fs::create_dir(&root)?;
    let root = std::fs::canonicalize(root)?;
    let missing = retained.path().join("missing");
    let config = Host {
        root: root.clone(),
        tools: missing.clone(),
        base_image: missing.clone(),
        census_serve: missing.clone(),
        restate: missing.clone(),
        captures: missing,
    };
    let error = crate::qualification_native_vm::host::execute(&config, &root)
        .err()
        .context("invalid socket root was accepted")?;
    assert!(
        error.to_string().contains("QEMU UNIX socket path too long"),
        "{error:#}"
    );
    assert_eq!(std::fs::read_dir(&root)?.count(), 0);
    assert!(root.is_dir());
    Ok(())
}
