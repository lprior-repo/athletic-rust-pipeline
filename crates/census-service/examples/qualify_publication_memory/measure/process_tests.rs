use super::*;
use anyhow::Context;
use std::io;

#[test]
fn output_capture_retains_only_budget_and_each_poll_is_finite() -> Result<()> {
    let mut capture = Capture::default();
    let mut infinite = io::repeat(b'x');
    for _ in 0..10 {
        capture.poll(&mut infinite)?;
    }
    ensure!(capture.bytes.len() == OUTPUT_LIMIT && capture.exceeded && !capture.eof);
    Ok(())
}

#[cfg(unix)]
fn script(directory: &Path, contents: &[u8]) -> Result<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let binary = directory.join("bounded-child.sh");
    std::fs::write(&binary, contents)?;
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700))?;
    Ok(binary)
}

#[cfg(unix)]
#[test]
fn timed_out_native_group_is_terminated_and_reaped() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let binary = script(directory.path(), b"#!/bin/sh\nwhile :; do sleep 1; done\n")?;
    let result = run(
        &binary,
        directory.path(),
        &directory.path().join("unused"),
        Duration::from_millis(100),
    )?;
    ensure!(result.timed_out && !result.success && !result.output_exceeded);
    ensure!(result.cleanup_error.is_none(), "{:?}", result.cleanup_error);
    ensure!(result.stdout.len() <= OUTPUT_LIMIT && result.stderr.len() <= OUTPUT_LIMIT);
    Ok(())
}

#[cfg(unix)]
#[test]
fn reaped_child_cleanup_does_not_signal_its_former_group() -> Result<()> {
    use std::os::unix::process::CommandExt;
    let mut child = OwnedChild {
        child: Command::new("/bin/sh")
            .args(["-c", "exit 0"])
            .process_group(0)
            .spawn()?,
        status: None,
        cleanup_attempted: false,
        cleanup_failure: None,
    };
    let started = Instant::now();
    while child.poll()?.is_none() && started.elapsed() < CLEANUP {
        std::thread::sleep(POLL);
    }
    ensure!(child.status.is_some());
    child.cleanup()?;
    child.cleanup()?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn inherited_output_writer_causes_bounded_refusal_without_joining() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let binary = script(directory.path(), b"#!/bin/sh\nsleep 3 &\nexit 0\n")?;
    let result = run(
        &binary,
        directory.path(),
        &directory.path().join("unused"),
        Duration::from_secs(5),
    )?;
    ensure!(!result.success);
    ensure!(result
        .cleanup_error
        .as_deref()
        .is_some_and(|error| error.contains("writers remain open")));
    Ok(())
}

#[cfg(unix)]
#[test]
fn term_ignoring_child_is_reported_unreaped_with_bounded_cleanup() -> Result<()> {
    use std::os::unix::process::CommandExt;
    let directory = tempfile::tempdir()?;
    let ready = directory.path().join("ready");
    let mut child = OwnedChild {
        child: Command::new("/bin/sh")
            .args(["-c", "trap '' TERM; : > \"$1\"; sleep 3", "fixture"])
            .arg(&ready)
            .process_group(0)
            .spawn()?,
        status: None,
        cleanup_attempted: false,
        cleanup_failure: None,
    };
    let started = Instant::now();
    while !ready.exists() && started.elapsed() < Duration::from_secs(1) {
        std::thread::sleep(POLL);
    }
    ensure!(ready.exists());
    let error = child
        .cleanup()
        .err()
        .context("TERM-ignoring child must refuse")?;
    ensure!(error.to_string().contains("unreaped"));
    let started = Instant::now();
    while child.poll()?.is_none() && started.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(POLL);
    }
    ensure!(child.status.is_some());
    Ok(())
}

#[cfg(unix)]
#[test]
fn group_signal_failure_survives_successful_reap_and_repeated_cleanup() -> Result<()> {
    let mut child = OwnedChild {
        child: Command::new("/bin/sh")
            .args(["-c", "read value"])
            .stdin(Stdio::piped())
            .spawn()?,
        status: None,
        cleanup_attempted: false,
        cleanup_failure: None,
    };
    let input = child.child.stdin.take().context("owned release input")?;
    let first = child
        .cleanup_with_after_signal(|| drop(input))
        .err()
        .context("missing group must fail")?;
    ensure!(first.to_string().contains("TERM failed"));
    ensure!(child.status.is_some());
    let second = child
        .cleanup()
        .err()
        .context("signal failure must persist")?;
    ensure!(first.to_string() == second.to_string());
    Ok(())
}

#[cfg(unix)]
#[test]
fn native_parent_interrupt_drains_owned_group() -> Result<()> {
    use rustix::process::Signal;
    if let Some(root) = std::env::var_os("PUBLICATION_MEMORY_SIGNAL_ROOT") {
        return assert_interrupted_child(Path::new(&root));
    }
    for (signal, name) in [(Signal::INT, "interrupt"), (Signal::TERM, "terminate")] {
        assert_native_parent_signal(signal, name)?;
    }
    Ok(())
}

#[cfg(unix)]
fn assert_interrupted_child(root: &Path) -> Result<()> {
    let expected = match std::env::var("PUBLICATION_MEMORY_SIGNAL_KIND")?.as_str() {
        "interrupt" => Interruption::Interrupt,
        "terminate" => Interruption::Terminate,
        other => anyhow::bail!("unexpected interruption: {other}"),
    };
    let binary = script(
        root,
        b"#!/bin/sh\n: > \"$PUBLICATION_MEMORY_SIGNAL_ROOT/ready\"\nexec sleep 5\n",
    )?;
    let outcome = run(&binary, root, &root.join("unused"), Duration::from_secs(10))?;
    ensure!(outcome.interruption == Some(expected));
    ensure!(!outcome.success && !outcome.timed_out && !outcome.output_exceeded);
    ensure!(
        outcome.cleanup_error.is_none(),
        "{:?}",
        outcome.cleanup_error
    );
    Ok(())
}

#[cfg(unix)]
fn assert_native_parent_signal(signal: rustix::process::Signal, name: &str) -> Result<()> {
    use rustix::process::{kill_process, Pid};
    use std::os::unix::process::CommandExt;
    let directory = tempfile::tempdir()?;
    let ready = directory.path().join("ready");
    let mut child = OwnedChild {
        child: Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "measure::process::tests::native_parent_interrupt_drains_owned_group",
                "--nocapture",
            ])
            .env("PUBLICATION_MEMORY_SIGNAL_ROOT", directory.path())
            .env("PUBLICATION_MEMORY_SIGNAL_KIND", name)
            .process_group(0)
            .spawn()?,
        status: None,
        cleanup_attempted: false,
        cleanup_failure: None,
    };
    let started = Instant::now();
    while !ready.exists() && started.elapsed() < Duration::from_secs(3) {
        ensure!(
            child.poll()?.is_none(),
            "signal fixture exited before readiness"
        );
        std::thread::sleep(POLL);
    }
    ensure!(ready.exists(), "signal fixture readiness timed out");
    ensure!(
        child.poll()?.is_none(),
        "signal fixture exited before signal"
    );
    kill_process(Pid::from_child(&child.child), signal)?;
    let started = Instant::now();
    while child.poll()?.is_none() && started.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(POLL);
    }
    ensure!(child.status.is_some_and(|status| status.success()));
    Ok(())
}
