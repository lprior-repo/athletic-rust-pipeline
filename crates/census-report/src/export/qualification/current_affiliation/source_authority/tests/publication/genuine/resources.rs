use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Serialize)]
struct DiskSample {
    logical_bytes: u64,
    allocated_bytes: u64,
    entries: usize,
}

fn sample(root: &Path) -> TestResult<DiskSample> {
    let mut pending = vec![root.to_path_buf()];
    let mut total = DiskSample {
        logical_bytes: 0,
        allocated_bytes: 0,
        entries: 0,
    };
    while let Some(path) = pending.pop() {
        total.entries = total.entries.checked_add(1).ok_or("entry count overflow")?;
        if total.entries > 4096 {
            return Err("retained resource sample exceeds 4096 entries".into());
        }
        let metadata = std::fs::symlink_metadata(&path)?;
        total.logical_bytes = total
            .logical_bytes
            .checked_add(metadata.len())
            .ok_or("logical disk count overflow")?;
        total.allocated_bytes = total
            .allocated_bytes
            .checked_add(allocated(&metadata)?)
            .ok_or("allocated disk count overflow")?;
        if metadata.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let admitted = total
                    .entries
                    .checked_add(pending.len())
                    .ok_or("sample worklist overflow")?;
                if admitted >= 4096 {
                    return Err("retained resource sample exceeds 4096 entries".into());
                }
                pending.try_reserve(1)?;
                pending.push(entry?.path());
            }
        }
    }
    Ok(total)
}

fn allocated(metadata: &std::fs::Metadata) -> TestResult<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(metadata
            .blocks()
            .checked_mul(512)
            .ok_or("allocated size overflow")?)
    }
    #[cfg(not(unix))]
    {
        let _ = metadata;
        Err("allocated disk sampling requires Unix metadata".into())
    }
}

fn peak_rss() -> TestResult<u64> {
    let status = std::fs::read_to_string("/proc/self/status")?;
    let value = status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .ok_or("VmHWM missing from process status")?;
    let mut fields = value.split_whitespace();
    let kib: u64 = fields.next().ok_or("VmHWM value missing")?.parse()?;
    if fields.next() != Some("kB") || fields.next().is_some() {
        return Err("unsupported VmHWM unit".into());
    }
    Ok(kib.checked_mul(1024).ok_or("VmHWM byte overflow")?)
}

fn digest(path: &Path) -> TestResult<String> {
    let mut input = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub(super) fn retain(
    case: &str,
    archive: tempfile::TempDir,
    output: tempfile::TempDir,
    frozen: &Path,
    workbook: &Path,
) -> TestResult<()> {
    if std::env::var_os("CENSUS_GENUINE_ROSTER_RETAIN").as_deref()
        != Some(std::ffi::OsStr::new("1"))
    {
        return Ok(());
    }
    let archive_disk = sample(archive.path())?;
    let output_disk = sample(output.path())?;
    let frozen_sha256 = digest(frozen)?;
    let workbook_sha256 = digest(workbook)?;
    let frozen_bytes = std::fs::metadata(frozen)?.len();
    let peak_rss_bytes = peak_rss()?;
    let archive_root = archive.keep();
    let output_root = output.keep();
    let report = json!({
        "case": case, "archive_root": archive_root, "output_root": output_root,
        "frozen_path": frozen, "frozen_sha256": frozen_sha256, "frozen_bytes": frozen_bytes,
        "workbook_path": workbook, "workbook_sha256": workbook_sha256,
        "archive_disk": archive_disk, "output_disk": output_disk,
        "peak_rss_bytes": peak_rss_bytes,
        "rss_scope": "Linux test-process VmHWM through completed public capture and cold publication replay",
        "disk_scope": "final owned archive/output sample before report; includes directory and symlink metadata, does not follow symlinks; excludes deleted Store/source roots; not peak disk"
    });
    let bytes = serde_json::to_vec(&report)?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(archive_root.join("genuine-retention.json"))?
        .write_all(&bytes)?;
    eprintln!("GENUINE_ROSTER_RETAIN {}", String::from_utf8(bytes)?);
    Ok(())
}
