use crate::report::{ExportResourceError, ReportResult};
use std::path::Path;

pub(crate) const MAX_OBJECT_BYTES: u64 =
    crate::export::qualification::physical_observations::OBSERVATION_BUDGET.bytes;

pub(crate) fn install(directory: &Path, bytes: &[u8]) -> ReportResult<String> {
    let length = super::codec::count(bytes.len()).map_err(super::codec_error)?;
    if length > MAX_OBJECT_BYTES {
        return Err(ExportResourceError::RowTooLarge {
            what: "frozen evidence proof object",
            bytes: length,
            max: MAX_OBJECT_BYTES,
        }
        .into());
    }
    super::objects::install(directory, bytes)
}

pub(crate) fn read(directory: &Path, digest: &str, ceiling: u64) -> ReportResult<Vec<u8>> {
    super::objects::read(directory, digest, ceiling.min(MAX_OBJECT_BYTES))
}
