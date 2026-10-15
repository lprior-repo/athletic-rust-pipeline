use super::TestResult;
use census_crawl::convert::ConverterObligation;
use census_crawl::convert::{
    converter_files_capped, converter_stdout_capped, ConverterLifecycleError, CONVERTER_DEADLINE,
    CONVERTER_MAX_OUTPUT_BYTES,
};
use census_crawl::CrawlError;
use std::ffi::OsStr;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[test]
fn converter_real_pdf_matches_the_captured_complete_text() -> TestResult {
    let actual = converter_stdout_capped(
        "pdftotext",
        ["-layout", "-", "-"],
        include_bytes!("../fixtures/mhsaa_enrollment/enrollment.pdf"),
        CONVERTER_DEADLINE,
        CONVERTER_MAX_OUTPUT_BYTES,
    )?;
    let actual = std::str::from_utf8(&actual)?;
    let expected = include_str!("../fixtures/mhsaa_enrollment/enrollment.txt");
    check!(eq; actual.split_whitespace().collect::<Vec<_>>(),
        expected.split_whitespace().collect::<Vec<_>>());
    Ok(())
}

#[test]
fn converter_empty_exact_and_excess_stdout_are_distinct() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let path = scratch.path().join("bytes");
    std::fs::write(&path, [])?;
    check!(eq; cat(&path, 0)?, Vec::<u8>::new());
    check!(eq; cat(&path, CONVERTER_MAX_OUTPUT_BYTES)?, Vec::<u8>::new());
    let bytes = b"\0fixture\xff";
    std::fs::write(&path, bytes)?;
    check!(eq; cat(&path, bytes.len())?, bytes);
    let limit = bytes
        .len()
        .checked_sub(1)
        .ok_or("fixture cannot be empty")?;
    check_resource(cat(&path, limit), bytes.len(), limit)?;
    check_resource(cat(&path, 0), 1, 0)
}

fn cat(path: &Path, limit: usize) -> Result<Vec<u8>, CrawlError> {
    converter_stdout_capped(
        "/bin/cat",
        [path.as_os_str()],
        b"",
        Duration::from_secs(10),
        limit,
    )
}

#[test]
fn converter_nonzero_exit_retains_the_real_status() -> TestResult {
    let outcome =
        converter_stdout_capped("/bin/sh", ["-c", "exit 9"], b"", Duration::from_secs(10), 1);
    let Err(CrawlError::Io { path, source }) = outcome else {
        return Err(format!("expected attributed exit error: {outcome:?}").into());
    };
    check!(eq; path, PathBuf::from("/bin/sh"));
    check!(
        source.to_string().contains("exit status: 9"),
        "status was not nine: {source}"
    );
    Ok(())
}

#[test]
fn converter_missing_watch_is_not_successful_empty_output() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let absent = scratch.path().join("absent");
    let outcome = converter_files_capped(
        "/bin/true",
        std::iter::empty::<&OsStr>(),
        &absent,
        Duration::from_secs(10),
        10,
    );
    let Err(CrawlError::Io { path, source }) = outcome else {
        return Err(format!("expected missing-watch refusal: {outcome:?}").into());
    };
    check!(eq; path, absent);
    check!(eq; source.kind(), std::io::ErrorKind::NotFound);
    Ok(())
}

#[test]
fn converter_created_watch_retains_exact_binary_file_contents() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let source = scratch.path().join("input");
    let output = scratch.path().join("output");
    let bytes = b"\0fixture\xff";
    std::fs::write(&source, bytes)?;
    converter_files_capped(
        "/bin/sh",
        [
            OsStr::new("-c"),
            OsStr::new("cat \"$1\" > \"$2\""),
            OsStr::new("converter-fixture"),
            source.as_os_str(),
            output.as_os_str(),
        ],
        &output,
        Duration::from_secs(10),
        bytes.len(),
    )?;
    check!(eq; std::fs::read(&output)?, bytes);
    Ok(())
}

#[test]
fn converter_watch_symlink_loop_is_attributed_and_cleanup_finishes() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let output = scratch.path().join("output");
    symlink("output", &output)?;
    let started = Instant::now();
    let outcome = converter_files_capped(
        "/bin/sh",
        ["-c", "exec sleep 300"],
        &output,
        Duration::from_secs(10),
        10,
    );
    let Err(CrawlError::Io { path, source }) = outcome else {
        return Err(format!("metadata error was hidden or cleanup failed: {outcome:?}").into());
    };
    check!(eq; path, output);
    check!(eq; source.raw_os_error(), Some(rustix::io::Errno::LOOP.raw_os_error()));
    check!(started.elapsed() < Duration::from_secs(5));
    Ok(())
}

#[test]
fn converter_deadline_does_not_wait_for_inherited_pipes() -> TestResult {
    let deadline = Duration::from_millis(100);
    let started = Instant::now();
    let outcome = converter_stdout_capped("/bin/sh", ["-c", "exec sleep 300"], b"", deadline, 10);
    let Err(CrawlError::ConverterDeadline {
        command,
        deadline: held,
    }) = outcome
    else {
        return Err(format!("expected cleanly finalized deadline: {outcome:?}").into());
    };
    check!(eq; command, "/bin/sh");
    check!(eq; held, deadline);
    check!(started.elapsed() < Duration::from_secs(5));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn converter_successful_leader_cannot_leave_descendant_stdout_join_hanging() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let pid_path = scratch.path().join("descendant.pid");
    let script =
        "sleep 300 & descendant=$!; printf '%s' \"$descendant\" > \"$1\"; printf 'fixture'; exit 0";
    let started = Instant::now();
    let outcome = converter_stdout_capped(
        "/bin/sh",
        [
            OsStr::new("-c"),
            OsStr::new(script),
            OsStr::new("converter-fixture"),
            pid_path.as_os_str(),
        ],
        b"",
        Duration::from_secs(10),
        10,
    );
    check!(
        started.elapsed() < Duration::from_secs(5),
        "descendant held the call open"
    );
    match outcome {
        Ok(bytes) => check!(eq; bytes, b"fixture"),
        Err(CrawlError::ConverterLifecycle(ConverterLifecycleError::Unproven {
            command,
            obligation: ConverterObligation::GroupAbsent,
            ..
        })) => check!(eq; command, "/bin/sh"),
        other => {
            return Err(format!(
                "expected bytes or explicit unproven descendant reaping: {other:?}"
            )
            .into())
        }
    }
    let pid = std::fs::read_to_string(&pid_path)?.parse::<u32>()?;
    let status_path = PathBuf::from(format!("/proc/{pid}/status"));
    match std::fs::read_to_string(&status_path) {
        Ok(status) => {
            let state = status
                .lines()
                .find(|line| line.starts_with("State:"))
                .ok_or("no process state")?;
            check!(
                state.contains("Z (zombie)"),
                "descendant remains alive: {state}"
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

#[test]
fn converter_rejects_envelope_before_spawn_or_open() -> TestResult {
    let requested = CONVERTER_MAX_OUTPUT_BYTES
        .checked_add(1)
        .ok_or("limit overflow")?;
    let missing = Path::new("/converter-fixture-path-that-must-not-be-opened");
    check_resource(
        converter_stdout_capped(
            "/converter-must-not-spawn",
            ["unused"],
            b"",
            CONVERTER_DEADLINE,
            requested,
        ),
        requested,
        CONVERTER_MAX_OUTPUT_BYTES,
    )?;
    check_resource(
        converter_files_capped(
            "/converter-must-not-spawn",
            ["unused"],
            missing,
            CONVERTER_DEADLINE,
            requested,
        ),
        requested,
        CONVERTER_MAX_OUTPUT_BYTES,
    )?;
    let requested = CONVERTER_DEADLINE
        .checked_add(Duration::from_secs(1))
        .ok_or("deadline overflow")?;
    for result in [
        converter_stdout_capped("/converter-must-not-spawn", ["unused"], b"", requested, 1)
            .map(drop),
        converter_files_capped(
            "/converter-must-not-spawn",
            ["unused"],
            missing,
            requested,
            1,
        ),
    ] {
        let Err(CrawlError::ConverterLifecycle(ConverterLifecycleError::DeadlineEnvelope {
            requested: held,
            limit,
        })) = result
        else {
            return Err(format!("invalid envelope reached spawning: {result:?}").into());
        };
        check!(eq; held, requested);
        check!(eq; limit, CONVERTER_DEADLINE);
    }
    Ok(())
}

fn check_resource<T: std::fmt::Debug>(
    result: Result<T, CrawlError>,
    requested: usize,
    limit: usize,
) -> TestResult {
    let Err(CrawlError::Resource {
        resource,
        requested: held,
        limit: cap,
    }) = result
    else {
        return Err(format!("expected exact resource refusal: {result:?}").into());
    };
    check!(eq; resource, "converter output bytes");
    check!(eq; held, requested);
    check!(eq; cap, limit);
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn converter_macos_successful_leader_resolves_inherited_stdout() -> TestResult {
    let started = Instant::now();
    let outcome = converter_stdout_capped(
        "/bin/sh",
        ["-c", "sleep 300 & printf 'fixture'; exit 0"],
        b"",
        Duration::from_millis(100),
        10,
    );
    check!(started.elapsed() < Duration::from_secs(5));
    match outcome {
        Ok(bytes) => check!(eq; bytes, b"fixture"),
        Err(CrawlError::ConverterLifecycle(ConverterLifecycleError::Unproven {
            command,
            obligation: ConverterObligation::GroupAbsent,
            ..
        })) => check!(eq; command, "/bin/sh"),
        other => return Err(format!("unexpected inherited-pipe outcome: {other:?}").into()),
    }
    Ok(())
}
