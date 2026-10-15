use crate::{CrawlError, CrawlResult};
use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

mod buffer;
mod error;
mod file;
#[cfg(all(
    unix,
    not(any(
        target_os = "cygwin",
        target_os = "emscripten",
        target_os = "espidf",
        target_os = "fuchsia",
        target_os = "horizon",
        target_os = "openbsd",
        target_os = "redox",
        target_os = "vita",
        target_os = "wasi"
    ))
))]
#[path = "convert/unix.rs"]
mod platform;
#[cfg(not(all(
    unix,
    not(any(
        target_os = "cygwin",
        target_os = "emscripten",
        target_os = "espidf",
        target_os = "fuchsia",
        target_os = "horizon",
        target_os = "openbsd",
        target_os = "redox",
        target_os = "vita",
        target_os = "wasi"
    ))
)))]
#[path = "convert/unsupported.rs"]
mod platform;

pub use error::{ConverterLifecycleError, ConverterObligation};

pub const CONVERTER_MAX_OUTPUT_BYTES: usize = 64 * 1024 * 1024;
pub const CONVERTER_DEADLINE: Duration = Duration::from_secs(120);

const CONVERTER_READ_CHUNK: usize = 64 * 1024;

pub fn converter_stdout_capped<A: AsRef<OsStr>>(
    command: &str,
    args: impl IntoIterator<Item = A>,
    input: &[u8],
    deadline: Duration,
    max_output: usize,
) -> CrawlResult<Vec<u8>> {
    validate(deadline, max_output)?;
    platform::stdout(command, args, input, deadline, max_output)
}

pub fn converter_files_capped<A: AsRef<OsStr>>(
    command: &str,
    args: impl IntoIterator<Item = A>,
    watch: &Path,
    deadline: Duration,
    max_output: usize,
) -> CrawlResult<()> {
    validate(deadline, max_output)?;
    platform::files(command, args, watch, deadline, max_output)
}

pub fn read_file_capped(path: &Path, max_bytes: usize) -> CrawlResult<Vec<u8>> {
    validate_bytes(max_bytes)?;
    file::read(path, max_bytes)
}

fn validate(deadline: Duration, max_output: usize) -> CrawlResult<()> {
    validate_bytes(max_output)?;
    if deadline > CONVERTER_DEADLINE {
        return Err(ConverterLifecycleError::DeadlineEnvelope {
            requested: deadline,
            limit: CONVERTER_DEADLINE,
        }
        .into());
    }
    Ok(())
}

fn validate_bytes(requested: usize) -> CrawlResult<()> {
    if requested > CONVERTER_MAX_OUTPUT_BYTES {
        return Err(CrawlError::Resource {
            resource: "converter output bytes",
            requested,
            limit: CONVERTER_MAX_OUTPUT_BYTES,
        });
    }
    Ok(())
}

fn converter_io(path: impl AsRef<Path>, source: std::io::Error) -> CrawlError {
    CrawlError::Io {
        path: path.as_ref().to_path_buf(),
        source,
    }
}

fn converter_bytes(limit: usize) -> CrawlError {
    CrawlError::Resource {
        resource: "converter output bytes",
        requested: limit.saturating_add(1),
        limit,
    }
}
