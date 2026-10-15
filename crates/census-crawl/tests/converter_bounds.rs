#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_crawl::convert::read_file_capped;
#[cfg(target_os = "linux")]
use census_crawl::convert::{converter_files_capped, converter_stdout_capped};
use census_crawl::CrawlError;
#[cfg(target_os = "linux")]
use std::path::Path;
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[path = "converter_bounds/file.rs"]
mod file;
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "converter_bounds/lifecycle.rs"]
mod lifecycle;

fn scratch_path(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("converter-bounds-{}-{tag}", std::process::id()))
}

#[cfg(target_os = "linux")]
fn read_pid(path: &Path, deadline: Duration) -> TestResult<i32> {
    let started = Instant::now();
    loop {
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Ok(pid) = text.trim().parse::<i32>() {
                return Ok(pid);
            }
        }
        if started.elapsed() >= deadline {
            return Err(format!("no converter pid was written to {}", path.display()).into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(target_os = "linux")]
fn assert_reaped(pid: i32) -> TestResult {
    check!(
        !PathBuf::from(format!("/proc/{pid}")).exists(),
        "the killed converter process {pid} must be reaped"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn pdftotext_output_over_cap_is_killed_and_reaped() -> TestResult {
    let pid_file = scratch_path("over-cap.pid");
    std::fs::remove_file(&pid_file).ok();
    let script = format!(
        "echo $$ > '{}'; while :; do printf 'xxxxxxxxxxxxxxxx'; done",
        pid_file.display()
    );
    let started = Instant::now();
    let outcome = converter_stdout_capped(
        "/bin/sh",
        ["-c", script.as_str()],
        b"",
        Duration::from_secs(60),
        4096,
    );
    check!(
        matches!(&outcome, Err(CrawlError::Resource { .. })),
        "converter output over the cap must be a typed resource refusal: {outcome:?}"
    );
    check!(
        started.elapsed() < Duration::from_secs(20),
        "an over-cap converter must be killed well before its deadline"
    );
    let pid = read_pid(&pid_file, Duration::from_secs(5))?;
    assert_reaped(pid)?;
    std::fs::remove_file(&pid_file)?;
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn a_converter_that_stalls_is_killed_and_reaped() -> TestResult {
    let pid_file = scratch_path("stall.pid");
    std::fs::remove_file(&pid_file).ok();
    let script = format!("echo $$ > '{}'; exec sleep 300", pid_file.display());
    let started = Instant::now();
    let outcome = converter_stdout_capped(
        "/bin/sh",
        ["-c", script.as_str()],
        b"",
        Duration::from_millis(300),
        4096,
    );
    check!(
        matches!(&outcome, Err(CrawlError::ConverterDeadline { .. })),
        "a stalled converter must be refused at its deadline: {outcome:?}"
    );
    check!(
        started.elapsed() < Duration::from_secs(20),
        "a stalled converter must be killed at its deadline, not left running"
    );
    let pid = read_pid(&pid_file, Duration::from_secs(5))?;
    assert_reaped(pid)?;
    std::fs::remove_file(&pid_file)?;
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn an_early_exiting_converter_retains_status_and_pending_writer_failure() -> TestResult {
    let payload = vec![b'x'; 4 * 1024 * 1024];
    let started = Instant::now();
    let outcome = converter_stdout_capped(
        "/bin/sh",
        ["-c", "exit 9"],
        payload.as_slice(),
        Duration::from_secs(60),
        4096,
    );
    check!(
        matches!(&outcome, Err(CrawlError::Io { .. }))
            || matches!(
                &outcome,
                Err(CrawlError::ConverterLifecycle(
                    census_crawl::convert::ConverterLifecycleError::Multiple { .. }
                ))
            ),
        "a converter that exits non-zero must fail: {outcome:?}"
    );
    check!(
        format!("{outcome:?}").contains("exited with"),
        "the failure must name the converter exit status: {outcome:?}"
    );
    check!(
        started.elapsed() < Duration::from_secs(20),
        "the writer must not outlive the converter that closed stdin"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn a_file_converter_over_its_watch_cap_is_killed_and_refused() -> TestResult {
    let output = scratch_path("watch.txt");
    std::fs::remove_file(&output).ok();
    let script = format!(
        "dd if=/dev/zero of='{}' bs=1024 count=256 status=none; while :; do :; done",
        output.display()
    );
    let started = Instant::now();
    let outcome = converter_files_capped(
        "/bin/sh",
        ["-c", script.as_str()],
        &output,
        Duration::from_secs(60),
        4096,
    );
    let primary = match outcome {
        Err(CrawlError::ConverterLifecycle(
            census_crawl::convert::ConverterLifecycleError::Multiple { primary, secondary },
        )) => {
            check!(
                matches!(
                    *secondary,
                    CrawlError::ConverterLifecycle(
                        census_crawl::convert::ConverterLifecycleError::Unproven {
                            obligation: census_crawl::convert::ConverterObligation::GroupAbsent,
                            ..
                        }
                    )
                ),
                "unexpected cleanup failure: {secondary:?}"
            );
            *primary
        }
        Err(error) => error,
        Ok(()) => return Err("over-cap file converter reported success".into()),
    };
    check!(
        matches!(
            &primary,
            CrawlError::Resource {
                resource: "converter output bytes",
                requested: 4097,
                limit: 4096,
            }
        ),
        "the primary failure must retain the exact byte-cap refusal: {primary:?}"
    );
    check!(
        started.elapsed() < Duration::from_secs(20),
        "an over-cap file converter must be killed well before its deadline"
    );
    std::fs::remove_file(&output)?;
    Ok(())
}

#[test]
fn a_converted_file_over_the_cap_is_refused_on_read() -> TestResult {
    let path = scratch_path("read-cap.txt");
    std::fs::write(&path, vec![b'a'; 8192])?;
    let outcome = read_file_capped(&path, 4096);
    check!(
        matches!(&outcome, Err(CrawlError::Resource { .. })),
        "a converted file over the cap must be a typed resource refusal: {outcome:?}"
    );
    let held = read_file_capped(&path, 8192)?;
    check!(eq; held.len(), 8192);
    std::fs::remove_file(&path)?;
    Ok(())
}
