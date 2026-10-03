use super::ExampleResult;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

const MAX_INSPECTION_BYTES: u64 = 4096;
const MAX_MOUNTINFO_BYTES: u64 = 1024 * 1024;
const MAX_FILESYSTEM_BYTES: u64 = 256 * 1024 * 1024;
const MAX_ERROR_DEPTH: usize = 32;

struct Mount<'a> {
    target: &'a Path,
    device: &'a str,
}

fn invalid(detail: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, detail)
}

fn parse_size(value: &str) -> io::Result<u64> {
    let (digits, multiplier) = match value.as_bytes().last() {
        Some(b'k' | b'K') => (value.strip_suffix(['k', 'K']), 1024_u64),
        Some(b'm' | b'M') => (value.strip_suffix(['m', 'M']), 1024_u64.pow(2)),
        Some(b'g' | b'G') => (value.strip_suffix(['g', 'G']), 1024_u64.pow(3)),
        _ => (Some(value), 1),
    };
    let digits = digits.ok_or_else(|| invalid("invalid filesystem size suffix"))?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid("filesystem size requires decimal digits"));
    }
    let size = digits
        .parse::<u64>()
        .map_err(|_| invalid("filesystem size exceeds u64"))?
        .checked_mul(multiplier)
        .ok_or_else(|| invalid("filesystem size multiplication overflow"))?;
    if size == 0 {
        return Err(invalid("filesystem size must be positive"));
    }
    Ok(size)
}

fn validate_device(device: &str) -> io::Result<()> {
    let (major, minor) = device
        .split_once(':')
        .ok_or_else(|| invalid("filesystem device is not major:minor"))?;
    [major, minor].into_iter().try_for_each(|part| {
        if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(invalid("filesystem device is not numeric"));
        }
        part.parse::<u32>()
            .map_err(|_| invalid("filesystem device exceeds u32"))?;
        Ok(())
    })
}

fn parse_mount(text: &str) -> io::Result<Mount<'_>> {
    if u64::try_from(text.len()).map_err(|_| invalid("inspection length overflow"))?
        > MAX_INSPECTION_BYTES
    {
        return Err(invalid("filesystem inspection exceeds byte budget"));
    }
    let mut lines = text.lines();
    let line = lines
        .next()
        .ok_or_else(|| invalid("no filesystem inspection"))?;
    if lines.next().is_some() {
        return Err(invalid(
            "filesystem inspection must identify exactly one mount",
        ));
    }
    let mut fields = line.split_whitespace();
    let values: [Option<&str>; 6] = std::array::from_fn(|_| fields.next());
    let [Some(kind), Some(options), Some(propagation), Some(target), Some(fsroot), Some(device)] =
        values
    else {
        return Err(invalid("filesystem inspection is missing fields"));
    };
    if fields.next().is_some() || kind != "tmpfs" || propagation != "private" || fsroot != "/" {
        return Err(invalid("fault medium must be a private whole tmpfs mount"));
    }
    let target = Path::new(target);
    if !target.is_absolute() || target == Path::new("/") {
        return Err(invalid(
            "fault medium must have a dedicated absolute mountpoint",
        ));
    }
    let mut sizes = options
        .split(',')
        .filter_map(|option| option.strip_prefix("size="));
    let size = parse_size(
        sizes
            .next()
            .ok_or_else(|| invalid("tmpfs size is missing"))?,
    )?;
    if sizes.next().is_some() || size > MAX_FILESYSTEM_BYTES {
        return Err(invalid(
            "tmpfs must have one explicit cap of at most 256 MiB",
        ));
    }
    validate_device(device)?;
    Ok(Mount { target, device })
}

fn reject_host_device(text: &str, device: &str) -> io::Result<()> {
    if text.is_empty() {
        return Err(invalid("host mount inspection is empty"));
    }
    text.lines().try_for_each(|line| {
        let host_device = line
            .split_whitespace()
            .nth(2)
            .ok_or_else(|| invalid("host mount inspection is malformed"))?;
        validate_device(host_device)?;
        if host_device == device {
            return Err(invalid(
                "fault medium is also mounted in the host namespace",
            ));
        }
        Ok(())
    })
}

fn stop_inspection(child: &mut Child) -> io::Result<()> {
    let killed = child.kill();
    let waited = child.wait();
    killed?;
    waited?;
    Ok(())
}

fn inspect_mount(parent: &Path) -> ExampleResult<String> {
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(4097)?;
    let mut child = Command::new("findmnt")
        .args([
            "-n",
            "-r",
            "-o",
            "FSTYPE,OPTIONS,PROPAGATION,TARGET,FSROOT,MAJ:MIN",
            "-T",
        ])
        .arg(parent)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let Some(stdout) = child.stdout.take() else {
        stop_inspection(&mut child)?;
        return Err(invalid("findmnt stdout is unavailable").into());
    };
    let read = stdout
        .take(MAX_INSPECTION_BYTES.saturating_add(1))
        .read_to_end(&mut bytes);
    if read.is_err() || bytes.len() > 4096 {
        stop_inspection(&mut child)?;
        read?;
        return Err(invalid("findmnt exceeded inspection byte budget").into());
    }
    read?;
    let status = child.wait()?;
    if !status.success() {
        return Err(io::Error::other(format!("findmnt inspection failed: {status}")).into());
    }
    Ok(String::from_utf8(bytes)?)
}

pub(super) fn inspect(root: &Path) -> ExampleResult<PathBuf> {
    let root = if root.is_absolute() {
        root.to_path_buf()
    } else {
        std::env::current_dir()?.join(root)
    };
    let name = root
        .file_name()
        .ok_or_else(|| invalid("store root has no filename"))?;
    let parent = root
        .parent()
        .ok_or_else(|| invalid("store root has no parent"))?;
    let parent = fs::canonicalize(parent)?;
    let text = inspect_mount(&parent)?;
    let mount = parse_mount(&text)?;
    let target = fs::canonicalize(mount.target)?;
    if !parent.starts_with(&target) {
        return Err(invalid("store parent is outside inspected tmpfs").into());
    }
    let mut host_bytes = Vec::new();
    host_bytes.try_reserve_exact(usize::try_from(MAX_MOUNTINFO_BYTES.saturating_add(1))?)?;
    File::open("/proc/1/mountinfo")?
        .take(MAX_MOUNTINFO_BYTES.saturating_add(1))
        .read_to_end(&mut host_bytes)?;
    if u64::try_from(host_bytes.len())? > MAX_MOUNTINFO_BYTES {
        return Err(invalid("host mount inspection exceeds byte budget").into());
    }
    reject_host_device(std::str::from_utf8(&host_bytes)?, mount.device)?;
    Ok(parent.join(name))
}

pub(super) fn is_enospc(error: &(dyn Error + 'static)) -> bool {
    std::iter::successors(Some(error), |current| (*current).source())
        .take(MAX_ERROR_DEPTH)
        .any(|current| {
            current
                .downcast_ref::<io::Error>()
                .is_some_and(|error| error.raw_os_error() == Some(28))
        })
}

#[cfg(test)]
#[path = "safety/tests.rs"]
mod tests;
