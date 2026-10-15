use super::ConverterLifecycleError;
use crate::CrawlResult;
use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

pub(super) fn stdout<A: AsRef<OsStr>>(
    command: &str,
    args: impl IntoIterator<Item = A>,
    input: &[u8],
    deadline: Duration,
    max_output: usize,
) -> CrawlResult<Vec<u8>> {
    drop((args, input, deadline, max_output));
    Err(ConverterLifecycleError::UnsupportedPlatform {
        command: command.to_owned(),
    }
    .into())
}

pub(super) fn files<A: AsRef<OsStr>>(
    command: &str,
    args: impl IntoIterator<Item = A>,
    watch: &Path,
    deadline: Duration,
    max_output: usize,
) -> CrawlResult<()> {
    drop((args, watch, deadline, max_output));
    Err(ConverterLifecycleError::UnsupportedPlatform {
        command: command.to_owned(),
    }
    .into())
}
