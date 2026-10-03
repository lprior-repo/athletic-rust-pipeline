use super::super::*;
use std::time::Instant;

#[test]
fn term_cancellation_interrupts_wait_and_joins_before_owner_exit() -> Result<()> {
    let root = tempfile::tempdir()?;
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args([
            "--exact",
            "qualification_native_vm::process::tests::signals::signal_region_fixture",
            "--nocapture",
        ])
        .env("QUALIFICATION_SIGNAL_FIXTURE", root.path());
    let mut fixture = Process::spawn(root.path(), "signal-fixture", &mut command)?;
    let ready = root.path().join("signal-ready.json");
    (0..1000)
        .find_map(|_| {
            if ready.exists() {
                return Some(Ok::<_, anyhow::Error>(()));
            }
            if let Err(error) = fixture.running() {
                return Some(Err(error));
            }
            std::thread::sleep(Duration::from_millis(10));
            None
        })
        .context("isolated signal fixture did not become ready")??;
    let fixture_pid = fixture
        .identity()
        .get("pid")
        .and_then(Value::as_u64)
        .context("fixture PID absent")?
        .to_string();
    command_for_signal(root.path(), &fixture_pid)?;
    assert_eq!(fixture.wait(150)?.code(), Some(0));
    let evidence = artifacts::json(&root.path().join("signal-cleanup.json"))?;
    assert_eq!(evidence.get("reason"), Some(&json!(15)));
    assert_eq!(evidence.get("wait_cancelled"), Some(&json!(true)));
    assert_eq!(evidence.pointer("/child/reaped"), Some(&json!(true)));
    assert_eq!(evidence.pointer("/child/exit/signal"), Some(&json!(15)));
    assert_eq!(
        evidence.pointer("/child/requested_signal"),
        Some(&json!(15))
    );
    let pid = evidence
        .pointer("/child/identity/pid")
        .and_then(Value::as_u64)
        .context("owned child PID absent")?;
    assert!(!Path::new(&format!("/proc/{pid}")).exists());
    Ok(())
}

fn command_for_signal(root: &Path, pid: &str) -> Result<()> {
    super::super::command(
        root,
        "fixture-term",
        Command::new("/usr/bin/kill").args(["-TERM", pid]),
        100,
    )?;
    Ok(())
}

#[test]
fn signal_region_fixture() -> Result<()> {
    let Some(root) = std::env::var_os("QUALIFICATION_SIGNAL_FIXTURE") else {
        return Ok(());
    };
    let root = PathBuf::from(root);
    if std::env::var_os("QUALIFICATION_LOG_SMOKE").is_some() {
        return capture_resource_smoke(&root);
    }
    let mut signals = cancellation::Signals::install()?;
    let mut child = Process::spawn(
        &root,
        "owned-sleep",
        Command::new("/usr/bin/sleep").arg("60"),
    )?;
    artifacts::publish(&root.join("signal-ready.json"), &child.identity())?;
    let started = Instant::now();
    let waited = child.wait(600);
    let stopped = child.stop();
    let reason = signals.reason();
    signals.finish()?;
    artifacts::publish(
        &root.join("signal-cleanup.json"),
        &json!({"reason":reason,"wait_cancelled":waited.is_err(),"wait":format!("{waited:?}"),"child":child.evidence(),"elapsed_ms":started.elapsed().as_millis()}),
    )?;
    stopped?;
    Ok(())
}

fn capture_resource_smoke(root: &Path) -> Result<()> {
    [("stdout", ""), ("stderr", " >&2")].into_iter().try_for_each(|(label, redirect)| -> Result<()> {
        let bytes = artifacts::LIMIT.checked_add(8192).context("log smoke length overflow")?;
        let script = format!("exec /usr/bin/head -c {bytes} /dev/zero{redirect}");
        let mut child = Process::spawn(root, label, Command::new("/bin/sh").args(["-c", &script]))?;
        let waited = child.wait(300);
        let stopped = child.stop();
        let path = if label == "stderr" { child.log.with_file_name(format!("stderr-{}.stderr.log", child.generation)) } else { child.log.clone() };
        let retained = artifacts::read(&path)?;
        let evidence = json!({"wait":format!("{waited:?}"),"cleanup":format!("{stopped:?}"),"child":child.evidence(),"log":path,"retained_bytes":retained.len(),"limit_bytes":artifacts::LIMIT,"resource_marked":retained.ends_with(capture::MARKER)});
        artifacts::publish(&root.join(format!("log-smoke-{label}.json")), &evidence)?;
        ensure!(waited.is_err() && stopped.is_err() && retained.ends_with(capture::MARKER), "log smoke did not expose resource failure: {evidence}");
        ensure!(evidence.pointer("/child/reaped") == Some(&json!(true)), "log smoke child not reaped: {evidence}");
        Ok(())
    })
}
