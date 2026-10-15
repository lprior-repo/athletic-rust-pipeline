#![cfg(target_os = "linux")]

#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_crawl::convert::{
    converter_files_capped, converter_stdout_capped, read_file_capped, CONVERTER_MAX_OUTPUT_BYTES,
};
use census_crawl::CrawlError;
use std::os::unix::fs::symlink;
use std::time::{Duration, Instant};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn converter_before_after_metadata_loop_is_not_zero_bytes() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let path = scratch.path().join("loop");
    symlink("loop", &path)?;
    let result = converter_files_capped(
        "/bin/true",
        std::iter::empty::<&str>(),
        &path,
        Duration::from_secs(10),
        1,
    );
    let Err(CrawlError::Io { path: held, source }) = result else {
        return Err(format!("metadata error was hidden: {result:?}").into());
    };
    check!(eq; held, path);
    check!(eq; source.raw_os_error(), Some(40));
    Ok(())
}

#[test]
fn converter_before_after_missing_watch_is_not_success() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let path = scratch.path().join("missing");
    let result = converter_files_capped(
        "/bin/true",
        std::iter::empty::<&str>(),
        &path,
        Duration::from_secs(10),
        1,
    );
    let Err(CrawlError::Io { path: held, source }) = result else {
        return Err(format!("missing output was treated as success: {result:?}").into());
    };
    check!(eq; held, path);
    check!(eq; source.kind(), std::io::ErrorKind::NotFound);
    Ok(())
}

#[test]
fn converter_before_after_successful_leader_does_not_join_a_live_descendant() -> TestResult {
    let started = Instant::now();
    let result = converter_stdout_capped(
        "/bin/sh",
        ["-c", "sleep 5 & printf 'fixture'; exit 0"],
        b"",
        Duration::from_millis(100),
        10,
    );
    check!(
        started.elapsed() < Duration::from_secs(3),
        "successful leader escaped the supervisor deadline while a descendant held stdout"
    );
    match result {
        Ok(bytes) => check!(eq; bytes, b"fixture"),
        Err(CrawlError::ConverterLifecycle(
            census_crawl::convert::ConverterLifecycleError::Unproven {
                obligation: census_crawl::convert::ConverterObligation::GroupAbsent,
                ..
            },
        )) => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

#[test]
fn converter_before_after_invalid_envelope_is_refused_before_file_open() -> TestResult {
    let requested = CONVERTER_MAX_OUTPUT_BYTES
        .checked_add(1)
        .ok_or("fixture overflow")?;
    let path = tempfile::tempdir()?.path().join("not-created");
    let result = read_file_capped(&path, requested);
    let Err(CrawlError::Resource {
        resource,
        requested: held,
        limit,
    }) = result
    else {
        return Err(format!("invalid envelope reached file open: {result:?}").into());
    };
    check!(eq; resource, "converter output bytes");
    check!(eq; held, requested);
    check!(eq; limit, CONVERTER_MAX_OUTPUT_BYTES);
    Ok(())
}
