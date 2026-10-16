use super::*;
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::Stdio;

const WORKER: &str = "workbook::publication::crash_tests::harness::tests::cleanup_worker";

#[test]
fn cleanup_worker() -> TestResult {
    let Some(mode) = std::env::var_os("CENSUS_PUBLICATION_CLEANUP_MODE") else {
        return Ok(());
    };
    if mode == "exit" {
        std::process::exit(23);
    }
    if mode == "idle" {
        return hold();
    }
    let socket = std::env::var_os("CENSUS_PUBLICATION_CLEANUP_SOCKET")
        .ok_or("cleanup fixture socket missing")?;
    let mut stream = UnixStream::connect(Path::new(&socket))?;
    match mode.to_str() {
        Some("partial") => {
            stream.write_all(b"in")?;
            stream.shutdown(std::net::Shutdown::Write)?;
        }
        Some("wrong") => stream.write_all(b"broken")?,
        Some("valid") => stream.write_all(b"intent")?,
        Some("silent") => {}
        _ => return Err("unknown cleanup fixture mode".into()),
    }
    hold()
}

fn hold() -> TestResult {
    let mut release = [0_u8; 1];
    std::io::stdin().read_exact(&mut release)?;
    Err("cleanup fixture unexpectedly released".into())
}

fn budget() -> WaitBudget {
    WaitBudget {
        polls: 1000,
        interval: Duration::from_millis(5),
        read_timeout: Duration::from_millis(100),
    }
}

fn command(mode: &str, socket: &Path) -> TestResult<Command> {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--exact", WORKER, "--nocapture"])
        .env("CENSUS_PUBLICATION_CLEANUP_MODE", mode)
        .env("CENSUS_PUBLICATION_CLEANUP_SOCKET", socket)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    Ok(command)
}

fn failed_checkpoint(mode: &str, budget: WaitBudget) -> TestResult<Box<ChildFailure>> {
    let directory = tempfile::tempdir_in("/tmp")?;
    let socket = directory.path().join("cleanup.sock");
    let listener = UnixListener::bind(&socket)?;
    let result = with_child(command(mode, &socket)?, |child| {
        wait_checkpoint(listener, child, "intent", budget)?;
        Ok(())
    });
    failure(result)
}

fn failure(result: TestResult) -> TestResult<Box<ChildFailure>> {
    match result {
        Ok(()) => Err("fixture unexpectedly succeeded".into()),
        Err(error) => error
            .downcast::<ChildFailure>()
            .map_err(|error| format!("missing child cleanup report: {error}").into()),
    }
}

fn check_killed(error: &ChildFailure) -> TestResult {
    let status = error
        .cleanup
        .status
        .as_ref()
        .map_err(|error| error.to_string())?;
    check!(eq; status.signal(), Some(9));
    check!(eq; error.cleanup.inspection_error.is_none(), true);
    check!(eq; error.cleanup.kill_error.is_none(), true);
    Ok(())
}

#[test]
fn checkpoint_timeout_terminates_and_reaps_owned_child() -> TestResult {
    let error = failed_checkpoint(
        "idle",
        WaitBudget {
            polls: 0,
            ..budget()
        },
    )?;
    check!(eq; error.primary.to_string(), "checkpoint accept timed out");
    check_killed(&error)
}

#[test]
fn early_exit_retains_original_status_and_is_reaped() -> TestResult {
    let error = failed_checkpoint("exit", budget())?;
    let early = error
        .primary
        .downcast_ref::<WaitFailure>()
        .ok_or("missing early exit")?;
    let WaitFailure::Exited(status) = early else {
        return Err("early exit became a timeout".into());
    };
    check!(eq; status.code(), Some(23));
    let reaped = error
        .cleanup
        .status
        .as_ref()
        .map_err(|error| error.to_string())?;
    check!(eq; reaped.code(), Some(23));
    check!(eq; error.cleanup.kill_error.is_none(), true);
    Ok(())
}

#[test]
fn truncated_checkpoint_preserves_read_error_and_reaps_child() -> TestResult {
    let error = failed_checkpoint("partial", budget())?;
    let read = error
        .primary
        .downcast_ref::<io::Error>()
        .ok_or("missing read error")?;
    check!(eq; read.kind(), io::ErrorKind::UnexpectedEof);
    check_killed(&error)
}

#[test]
fn wrong_checkpoint_preserves_validation_error_and_reaps_child() -> TestResult {
    let error = failed_checkpoint("wrong", budget())?;
    let validation = error
        .primary
        .downcast_ref::<io::Error>()
        .ok_or("missing validation error")?;
    check!(eq; validation.kind(), io::ErrorKind::InvalidData);
    check!(eq; validation.to_string(), "checkpoint mismatch: expected [105, 110, 116, 101, 110, 116], received [98, 114, 111, 107, 101, 110]");
    check_killed(&error)
}

#[test]
fn silent_checkpoint_read_timeout_reaps_owned_child() -> TestResult {
    let error = failed_checkpoint("silent", budget())?;
    let read = error
        .primary
        .downcast_ref::<io::Error>()
        .ok_or("missing read timeout")?;
    check!(eq; matches!(read.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut), true);
    check_killed(&error)
}

#[test]
fn later_callback_error_keeps_original_source_and_cleanup_status() -> TestResult {
    let directory = tempfile::tempdir_in("/tmp")?;
    let socket = directory.path().join("cleanup.sock");
    let listener = UnixListener::bind(&socket)?;
    let error = failure(with_child(command("valid", &socket)?, |child| {
        wait_checkpoint(listener, child, "intent", budget())?;
        Err(io::Error::new(io::ErrorKind::PermissionDenied, "fixture action denied").into())
    }))?;
    let original = error
        .primary
        .downcast_ref::<io::Error>()
        .ok_or("missing original error")?;
    check!(eq; original.kind(), io::ErrorKind::PermissionDenied);
    check!(eq; std::error::Error::source(error.as_ref()).map(ToString::to_string), Some("fixture action denied".to_owned()));
    check_killed(&error)
}

#[test]
fn already_reaped_success_keeps_callback_value() -> TestResult {
    let directory = tempfile::tempdir_in("/tmp")?;
    let socket = directory.path().join("cleanup.sock");
    let result = with_child(command("exit", &socket)?, |child| {
        let status = child.wait()?;
        check!(eq; status.code(), Some(23));
        Ok("original success")
    })?;
    check!(eq; result, "original success");
    Ok(())
}

#[test]
fn cleanup_errors_do_not_replace_the_original_error() -> TestResult {
    let cleanup = Cleanup {
        status: Err(io::Error::other("wait failure")),
        inspection_error: Some(io::Error::other("inspection failure")),
        kill_error: Some(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "termination denied",
        )),
    };
    let error = failure(finish(
        Err(io::Error::new(io::ErrorKind::UnexpectedEof, "original read failure").into()),
        cleanup,
    ))?;
    let original = error
        .primary
        .downcast_ref::<io::Error>()
        .ok_or("missing original error")?;
    check!(eq; original.kind(), io::ErrorKind::UnexpectedEof);
    check!(eq; original.to_string(), "original read failure");
    check!(eq; error.cleanup.status.as_ref().err().map(ToString::to_string), Some("wait failure".to_owned()));
    check!(eq; error.cleanup.inspection_error.as_ref().map(ToString::to_string), Some("inspection failure".to_owned()));
    check!(eq; error.cleanup.kill_error.as_ref().map(ToString::to_string), Some("termination denied".to_owned()));
    Ok(())
}

#[test]
fn successful_callback_cannot_hide_cleanup_failure() -> TestResult {
    let cleanup = Cleanup {
        status: Err(io::Error::other("wait failure")),
        inspection_error: None,
        kill_error: None,
    };
    let error = failure(finish(Ok(()), cleanup))?;
    check!(eq; error.primary.to_string(), "fixture callback succeeded but child cleanup failed");
    check!(eq; error.cleanup.status.as_ref().err().map(ToString::to_string), Some("wait failure".to_owned()));
    Ok(())
}

#[test]
fn sigterm_timeout_reaps_owned_child_without_kill() -> TestResult {
    let directory = tempfile::tempdir_in("/tmp")?;
    let root = directory.path().to_path_buf();
    let socket = root.join("cleanup.sock");
    let listener = UnixListener::bind(&socket)?;
    let result = with_terminated_child(command("idle", &socket)?, directory, |child| {
        wait_checkpoint(
            listener,
            child,
            "intent",
            WaitBudget {
                polls: 0,
                ..budget()
            },
        )?;
        Ok(())
    })
    .map(|_| ());
    let error = failure(result)?;
    check!(eq; error.primary.to_string(), "checkpoint accept timed out");
    let status = error
        .cleanup
        .status
        .as_ref()
        .map_err(|error| error.to_string())?;
    check!(eq; status.signal(), Some(15));
    check!(eq; error.cleanup.inspection_error.is_none(), true);
    check!(eq; error.cleanup.kill_error.is_none(), true);
    check!(eq; root.exists(), false);
    Ok(())
}

#[test]
fn sigterm_wrong_checkpoint_keeps_read_failure_and_reaps_child() -> TestResult {
    let directory = tempfile::tempdir_in("/tmp")?;
    let socket = directory.path().join("cleanup.sock");
    let listener = UnixListener::bind(&socket)?;
    let result = with_terminated_child(command("wrong", &socket)?, directory, |child| {
        wait_checkpoint(listener, child, "intent", budget())?;
        Ok(())
    })
    .map(|_| ());
    let error = failure(result)?;
    let primary = error
        .primary
        .downcast_ref::<io::Error>()
        .ok_or("checkpoint error")?;
    check!(eq; primary.kind(), io::ErrorKind::InvalidData);
    let status = error
        .cleanup
        .status
        .as_ref()
        .map_err(|error| error.to_string())?;
    check!(eq; status.signal(), Some(15));
    check!(eq; error.cleanup.inspection_error.is_none(), true);
    check!(eq; error.cleanup.kill_error.is_none(), true);
    Ok(())
}

#[test]
fn sigterm_already_reaped_child_keeps_value_and_owned_directory() -> TestResult {
    let directory = tempfile::tempdir_in("/tmp")?;
    let root = directory.path().to_path_buf();
    let socket = root.join("cleanup.sock");
    let (value, directory) =
        with_terminated_child(command("exit", &socket)?, directory, |child| {
            check!(eq; child.wait()?.code(), Some(23));
            Ok("selected child result")
        })?;
    check!(eq; value, "selected child result");
    check!(eq; directory.path(), root.as_path());
    check!(eq; root.is_dir(), true);
    directory.close()?;
    check!(eq; root.exists(), false);
    Ok(())
}
