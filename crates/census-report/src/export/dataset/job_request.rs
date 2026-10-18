use super::{frozen, ExportDataset};
use crate::report::ReportResult;
use census_store::Store;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "contract", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExportJobRequest {
    LegacyV2,
    SelectedPublicationV1 {
        selection: census_domain::model::ProfileSelectionV1,
    },
}

impl ExportDataset {
    pub fn for_job_request(
        store: &Store,
        job: &str,
        request: &ExportJobRequest,
    ) -> ReportResult<Self> {
        frozen::job::capture(store, job, request)
    }

    pub fn for_job(store: &Store, job: &str) -> ReportResult<Self> {
        Self::for_job_request(store, job, &ExportJobRequest::LegacyV2)
    }
}
