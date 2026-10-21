use super::{freshness::Freshness, invalid, io_error, ExportDataset, ReportResult, Store};
use crate::export::ExportJobRequest;
use census_domain::model::ProfileSelectionV1;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;

const MAX_BYTES: u64 = 16 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Commitment {
    request: ExportJobRequest,
    job: String,
    store_identity: String,
    pub(super) archive_digest: String,
    pub(super) freshness: Freshness,
}

impl Commitment {
    pub(super) fn new(
        job: &str,
        selection: &ProfileSelectionV1,
        dataset: &ExportDataset,
        freshness: Freshness,
    ) -> ReportResult<Self> {
        Ok(Self {
            request: ExportJobRequest::SelectedPublicationV1 {
                selection: selection.clone(),
            },
            job: job.to_owned(),
            store_identity: dataset.lineage.store_identity.clone(),
            archive_digest: dataset.archive_digest()?,
            freshness,
        })
    }

    pub(super) fn require_request(
        &self,
        store: &Store,
        job: &str,
        selection: &ProfileSelectionV1,
    ) -> ReportResult<()> {
        let wanted = ExportJobRequest::SelectedPublicationV1 {
            selection: selection.clone(),
        };
        if self.request != wanted
            || self.job != job
            || self.store_identity != super::super::store_identity(store)?
        {
            return Err(invalid(
                "selected export job request, identity or store mismatch",
            ));
        }
        if !super::super::super::hex_digest(&self.archive_digest) {
            return Err(invalid(
                "selected export job has a malformed archive commitment",
            ));
        }
        Ok(())
    }

    pub(super) fn write(&self, path: &Path) -> ReportResult<()> {
        let bytes =
            serde_json::to_vec(self).map_err(|source| crate::report::ReportError::Decode {
                path: path.into(),
                line: 0,
                source,
            })?;
        if u64::try_from(bytes.len()).map_err(|_| invalid("selected commitment length overflow"))?
            > MAX_BYTES
        {
            return Err(invalid("selected export commitment exceeds its byte limit"));
        }
        super::super::super::io::write_json(path, self)
    }

    pub(super) fn read(path: &Path) -> ReportResult<Self> {
        let metadata = std::fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
        if !metadata.file_type().is_file() || metadata.len() > MAX_BYTES {
            return Err(invalid(
                "selected export commitment is not a bounded regular file",
            ));
        }
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|source| io_error(path, source))?
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|source| io_error(path, source))?;
        if u64::try_from(bytes.len()).map_err(|_| invalid("selected commitment length overflow"))?
            > MAX_BYTES
        {
            return Err(invalid("selected export commitment exceeds its byte limit"));
        }
        let value: Self = serde_json::from_slice(&bytes).map_err(|source| {
            crate::report::ReportError::Decode {
                path: path.into(),
                line: 0,
                source,
            }
        })?;
        let canonical =
            serde_json::to_vec(&value).map_err(|source| crate::report::ReportError::Decode {
                path: path.into(),
                line: 0,
                source,
            })?;
        if canonical != bytes {
            return Err(invalid("selected export commitment is not canonical"));
        }
        Ok(value)
    }
}
