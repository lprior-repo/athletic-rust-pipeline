use super::{temporaries, TestResult};
use crate::school_address::replay::capture::{hex, preserve, ReplayError};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const PARTIAL_WRITE_CHILD: &str = "SOL_CAPTURE_PARTIAL_WRITE_CHILD";
const KILL_CHILD_OUT: &str = "SOL_CAPTURE_KILL_CHILD_OUT";
const PARTIAL_WRITE_TEST: &str = "school_address::replay::capture::tests::interruption::a_returned_partial_capture_write_error_cleans_the_stage_and_the_exact_retry_succeeds";
const KILL_TEST: &str = "school_address::replay::capture::tests::interruption::a_killed_capture_writer_leaves_no_published_object_and_the_retry_reuses_completed_objects";
const WRITE_LIMIT_BLOCKS: u64 = 8;

#[cfg(unix)]
#[test]
fn a_returned_partial_capture_write_error_cleans_the_stage_and_the_exact_retry_succeeds(
) -> TestResult {
    if let Ok(dir) = std::env::var(PARTIAL_WRITE_CHILD) {
        return partial_write_child(Path::new(&dir));
    }
    let dir = TempDir::new()?;
    let output = limited_child(
        PARTIAL_WRITE_TEST,
        true,
        WRITE_LIMIT_BLOCKS,
        &[(PARTIAL_WRITE_CHILD, dir.path())],
    )?;
    check!(
        eq;
        output.status.code(),
        Some(77),
        "the partial write child did not return the write error: {}",
        child_report(&output)
    );
    let payload = vec![0x5A_u8; 64 * 1024];
    for name in ["manifest.json", "capture.bin"] {
        let target = dir.path().join(name);
        check!(
            !target.exists(),
            "{name} was published from a partial write"
        );
        preserve(&target, &payload)?;
        check!(eq; std::fs::read(&target)?, payload);
    }
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

fn partial_write_child(dir: &Path) -> TestResult {
    let payload = vec![0x5A_u8; 64 * 1024];
    for name in ["manifest.json", "capture.bin"] {
        let target = dir.join(name);
        match preserve(&target, &payload) {
            Err(ReplayError::Io { path, source }) => {
                check!(
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.contains(".tmp.")),
                    "the partial write error does not name a staged temporary: {}",
                    path.display()
                );
                check!(
                    eq;
                    source.raw_os_error(),
                    Some(27),
                    "the staged write did not return EFBIG: {source:?}"
                );
            }
            other => {
                return Err(format!("{name}: the partial write was not returned: {other:?}").into())
            }
        }
        check!(
            !target.exists(),
            "{name} was published from a partial write"
        );
    }
    check!(eq; temporaries(dir)?, Vec::<String>::new());
    std::process::exit(77)
}

#[cfg(unix)]
#[test]
fn a_killed_capture_writer_leaves_no_published_object_and_the_retry_reuses_completed_objects(
) -> TestResult {
    if let Ok(out) = std::env::var(KILL_CHILD_OUT) {
        return killed_child(Path::new(&out));
    }
    check_killed_variant(KillPoint::Manifest)?;
    check_killed_variant(KillPoint::Body)?;
    Ok(())
}

#[derive(Clone, Copy)]
enum KillPoint {
    Manifest,
    Body,
}

#[cfg(unix)]
struct Fixture {
    manifest: PathBuf,
    bytes: Vec<u8>,
    captures: Vec<(PathBuf, u64)>,
}

#[cfg(unix)]
fn fixture() -> TestResult<Fixture> {
    let manifest = fixture_manifest();
    let base = manifest
        .parent()
        .ok_or("the fixture manifest has no parent")?
        .to_path_buf();
    let bytes = std::fs::read(&manifest)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let listed = value["captures"]
        .as_array()
        .ok_or("the fixture manifest has no captures")?;
    let mut captures = Vec::new();
    for capture in listed {
        let relative = capture["path"]
            .as_str()
            .ok_or("a fixture capture has no path")?;
        let source = base.join(relative);
        captures.push((source.clone(), std::fs::metadata(&source)?.len()));
    }
    Ok(Fixture {
        manifest,
        bytes,
        captures,
    })
}

#[cfg(unix)]
fn fixture_manifest() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../census-crawl/tests/fixtures/school_address_replay/manifest.json")
}

#[cfg(unix)]
fn check_killed_variant(point: KillPoint) -> TestResult {
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::process::ExitStatusExt;

    let fixture = fixture()?;
    let limit_blocks = kill_limit_blocks(point, &fixture)?;
    let dir = TempDir::new()?;
    let out = dir.path().join("corpus");
    let archive = out.join("captures").join(hex(&fixture.bytes));
    let output = limited_child(
        KILL_TEST,
        false,
        limit_blocks,
        &[(KILL_CHILD_OUT, out.as_path())],
    )?;
    check!(
        eq;
        output.status.signal(),
        Some(25),
        "the capture writer was not killed mid-write: {}",
        child_report(&output)
    );
    check!(archive.is_dir(), "{} is missing", archive.display());
    let staged = temporaries(&archive)?;
    check!(eq; staged.len(), 1, "unexpected staged temporaries: {staged:?}");
    let partial = staged.first().ok_or("the interrupted stage is missing")?;
    check!(eq; std::fs::metadata(archive.join(partial))?.len(), limit_blocks * 512);
    check!(!archive.join(".complete").exists());
    match point {
        KillPoint::Manifest => check!(eq; published(&archive)?, Vec::<String>::new()),
        KillPoint::Body => check!(eq; published(&archive)?, vec!["manifest.json".to_string()]),
    }
    let completed = match point {
        KillPoint::Manifest => None,
        KillPoint::Body => Some(std::fs::metadata(archive.join("manifest.json"))?.ino()),
    };
    let replayed = crate::school_address::replay::run(&fixture.manifest, &out)?;
    check!(eq; replayed.archive(), archive.as_path());
    check!(archive.join(".complete").is_file());
    check!(eq; std::fs::read(archive.join("manifest.json"))?, fixture.bytes);
    if let Some(inode) = completed {
        check!(eq; std::fs::metadata(archive.join("manifest.json"))?.ino(), inode);
    }
    for (source, _) in &fixture.captures {
        let bytes = std::fs::read(source)?;
        check!(eq; std::fs::read(archive.join(hex(&bytes)))?, bytes);
    }
    check!(eq; temporaries(&archive)?, staged);
    Ok(())
}

#[cfg(unix)]
fn kill_limit_blocks(point: KillPoint, fixture: &Fixture) -> TestResult<u64> {
    let manifest = u64::try_from(fixture.bytes.len())
        .map_err(|_| "the fixture manifest size is not representable")?;
    let limit = match point {
        KillPoint::Manifest => manifest.saturating_sub(1) / 512 * 512,
        KillPoint::Body => manifest.div_ceil(512) * 512,
    };
    check!(
        limit >= 512,
        "the fixture manifest is too small for an isolated kill point"
    );
    match point {
        KillPoint::Manifest => check!(
            manifest > limit,
            "the fixture manifest does not exceed the kill limit"
        ),
        KillPoint::Body => {
            let first = fixture
                .captures
                .first()
                .ok_or("the fixture manifest has no captures")?;
            check!(
                first.1 > limit,
                "the first fixture capture does not exceed the body kill limit"
            );
        }
    }
    Ok(limit / 512)
}

fn published(dir: &Path) -> TestResult<Vec<String>> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let name = entry?
            .file_name()
            .to_str()
            .map(str::to_string)
            .ok_or("published filename is not UTF-8")?;
        if !name.contains(".tmp.") {
            found.push(name);
        }
    }
    found.sort();
    Ok(found)
}

#[cfg(unix)]
fn limited_child(
    test: &str,
    ignore_file_size_signal: bool,
    limit_blocks: u64,
    envs: &[(&str, &Path)],
) -> TestResult<std::process::Output> {
    let ignore = if ignore_file_size_signal {
        "trap '' 25; "
    } else {
        ""
    };
    let script = format!("{ignore}ulimit -f {limit_blocks}; exec \"$@\"");
    let mut command = std::process::Command::new("sh");
    command
        .arg("-c")
        .arg(script)
        .arg("sh")
        .arg(std::env::current_exe()?)
        .args(["--exact", test, "--nocapture"]);
    for &(name, value) in envs {
        command.env(name, value);
    }
    Ok(command.output()?)
}

#[cfg(unix)]
fn killed_child(out: &Path) -> TestResult {
    match crate::school_address::replay::run(&fixture_manifest(), out) {
        Ok(_) => Err("the interrupted replay completed".into()),
        Err(error) => Err(format!(
            "the interrupted replay returned an error instead of dying: {error}"
        )
        .into()),
    }
}

fn child_report(output: &std::process::Output) -> String {
    format!(
        "status={:?} stdout={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
