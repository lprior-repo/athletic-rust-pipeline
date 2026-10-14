use super::*;
use std::io::Write;
use std::os::unix::process::ExitStatusExt;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const UUID: &[u8; 37] = b"00112233-4455-6677-8899-aabbccddeeff\n";

fn shell(script: &str) -> Command {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", script]);
    command
}

fn failure<T>(result: io::Result<T>) -> TestResult<io::Error> {
    match result {
        Ok(_) => Err("boot identity unexpectedly succeeded".into()),
        Err(error) => Ok(error),
    }
}

#[test]
fn uuid_preserves_native_casing_and_newline() -> TestResult {
    for bytes in [UUID.to_vec(), UUID.to_ascii_uppercase()] {
        check!(eq; parse_boot_session(&bytes)?.as_slice(), bytes.as_slice());
        let expected = format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(&bytes));
        check!(eq; super::super::hash_boot_bytes(&bytes)?.as_str(), expected);
    }
    Ok(())
}

#[test]
fn uuid_rejects_malformed_empty_nil_and_excess_output() -> TestResult {
    let malformed: &[&[u8]] = &[
        b"",
        b"00112233-4455-6677-8899-aabbccddeeff",
        b"00112233-4455-6677-8899-aabbccddeeff\r\n",
        b"00112233-4455-6677-8899-aabbccddeeff\n\n",
        b" 00112233-4455-6677-8899-aabbccddeeff\n",
        b"00112233-4455-6677-8899-aabbccddeefg\n",
        b"0011223344455-6677-8899-aabbccddeeff\n",
        b"00000000-0000-0000-0000-000000000000\n",
        &[0xff; 37],
    ];
    for bytes in malformed {
        check!(eq; failure(parse_boot_session(bytes))?.kind(), io::ErrorKind::InvalidData);
    }
    Ok(())
}

#[test]
fn command_preserves_complete_output() -> TestResult {
    let bytes = run_command(
        shell("printf '00112233-4455-6677-8899-aabbccddeeff\\n'"),
        EXECUTION,
    )?;
    check!(eq; &bytes, UUID);
    Ok(())
}

#[test]
fn command_reports_spawn_failure_and_nonzero_exit() -> TestResult {
    let directory = tempfile::tempdir()?;
    let missing = Command::new(directory.path().join("missing-sysctl"));
    check!(eq; failure(run_command(missing, EXECUTION))?.kind(), io::ErrorKind::NotFound);
    let error = failure(run_command(shell("exit 7"), EXECUTION))?;
    check!(error.to_string().contains("exited with"));
    check!(error.to_string().contains('7'));
    Ok(())
}

#[test]
fn command_rejects_truncation_and_output_overflow() -> TestResult {
    for script in [
        "printf invalid",
        "printf '00112233-4455-6677-8899-aabbccddeeff\\nX'",
    ] {
        check!(eq; failure(run_command(shell(script), EXECUTION))?.kind(), io::ErrorKind::InvalidData);
    }
    Ok(())
}

#[test]
fn fragmented_output_requires_complete_uuid_and_eof() -> TestResult {
    let (mut reader, mut writer) = UnixStream::pair()?;
    reader.set_nonblocking(true)?;
    let mut bytes = [0_u8; 38];
    let mut length = 0;
    for chunk in UUID.chunks(7) {
        writer.write_all(chunk)?;
        check!(!read_chunk(&mut reader, &mut bytes, &mut length)?);
    }
    check!(eq; length, BOOT_BYTES);
    drop(writer);
    check!(read_chunk(&mut reader, &mut bytes, &mut length)?);
    check!(eq; &parse_boot_session(bytes.get(..length).ok_or("missing output")?)?, UUID);
    Ok(())
}

#[test]
fn exited_child_with_held_stdout_has_bounded_wait() -> TestResult {
    let (reader, mut writer) = UnixStream::pair()?;
    reader.set_nonblocking(true)?;
    writer.write_all(UUID)?;
    let mut child = OwnedChild(shell("exit 0").spawn()?);
    wait_reaped(&mut child.0)?;
    let error = failure(supervise(
        &mut child.0,
        reader,
        WaitBudget {
            polls: 2,
            interval: Duration::from_millis(1),
        },
    ))?;
    drop(writer);
    check!(eq; error.kind(), io::ErrorKind::TimedOut);
    Ok(())
}

#[test]
fn command_timeout_is_bounded() -> TestResult {
    let error = failure(run_command(
        shell("exec /bin/sleep 30"),
        WaitBudget {
            polls: 0,
            interval: Duration::from_millis(1),
        },
    ))?;
    check!(eq; error.kind(), io::ErrorKind::TimedOut);
    check!(eq; error.to_string(), "boot identity command timed out");
    Ok(())
}

#[test]
fn cleanup_terminates_with_sigterm_and_reaps() -> TestResult {
    let mut child = OwnedChild(Command::new("/bin/sleep").arg("30").spawn()?);
    terminate_and_reap(&mut child.0)?;
    let status = child.0.try_wait()?.ok_or("child was not reaped")?;
    check!(eq; status.signal(), Some(15));
    Ok(())
}

#[test]
fn cleanup_inspection_failure_preserves_error_without_signaling() -> TestResult {
    let mut child = OwnedChild(shell("exit 0").spawn()?);
    let pid = Pid::from_child(&child.0);
    for _ in 0..1000 {
        if rustix::process::waitpid(Some(pid), rustix::process::WaitOptions::NOHANG)?.is_some() {
            let error = failure(terminate_and_reap(&mut child.0))?;
            check!(eq; error.raw_os_error(), Some(rustix::io::Errno::CHILD.raw_os_error()));
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Err("fixture child did not exit".into())
}

#[test]
fn cleanup_runs_during_unwind() -> TestResult {
    let child = OwnedChild(Command::new("/bin/sleep").arg("30").spawn()?);
    let pid = Pid::from_child(&child.0);
    let unwind = std::panic::catch_unwind(move || {
        let _owned = child;
        std::panic::resume_unwind(Box::new("boot identity cleanup fixture"));
    });
    check!(unwind.is_err());
    check!(matches!(
        rustix::process::waitpid(Some(pid), rustix::process::WaitOptions::NOHANG),
        Err(rustix::io::Errno::CHILD)
    ));
    Ok(())
}

#[test]
fn cleanup_failure_retains_primary_error() -> TestResult {
    let primary = io::Error::new(io::ErrorKind::TimedOut, "command deadline");
    let cleanup = io::Error::other("reap failed");
    let error = failure(finish::<()>(Err(primary), Err(cleanup)))?;
    check!(eq; error.kind(), io::ErrorKind::TimedOut);
    check!(eq; error.to_string(), "command deadline; child cleanup failed: reap failed");
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn linux_hashes_exact_existing_proc_bytes() -> TestResult {
    let bytes = std::fs::read("/proc/sys/kernel/random/boot_id")?;
    let expected = format!("{:x}", <sha2::Sha256 as sha2::Digest>::digest(&bytes));
    check!(eq; super::super::read_owner_boot_digest()?.as_str(), expected);
    let lease = crate::LeaseRecord::new("owner", "2999-12-31", 1, &"a".repeat(64))?;
    let json = serde_json::to_value(lease)?;
    check!(eq; json.get("owner_boot").and_then(serde_json::Value::as_str), Some(expected.as_str()));
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn macos_hashes_stable_genuine_boot_session_bytes() -> TestResult {
    let mut command = Command::new(SYSCTL);
    command.args(["-n", "kern.bootsessionuuid"]);
    let bytes = run_command(command, EXECUTION)?;
    let expected = hash_boot_bytes(&bytes)?;
    check!(eq; read_owner_boot_digest()?, expected);
    check!(eq; super::super::read_owner_boot_digest()?, expected);
    let lease = crate::LeaseRecord::new("owner", "2999-12-31", 1, &"a".repeat(64))?;
    let json = serde_json::to_value(lease)?;
    check!(eq; json.get("owner_boot").and_then(serde_json::Value::as_str), Some(expected.as_str()));
    Ok(())
}
