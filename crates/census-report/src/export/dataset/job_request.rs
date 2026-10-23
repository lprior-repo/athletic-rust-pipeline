use super::{frozen, ExportDataset};
use crate::report::ReportResult;
use census_store::Store;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "contract", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExportJobRequest {
    SelectedPublicationV1 {
        selection: census_domain::model::ProfileSelectionV1,
    },
}

impl ExportJobRequest {
    pub fn require_selected_publication(&self) -> &census_domain::model::ProfileSelectionV1 {
        let Self::SelectedPublicationV1 { selection } = self;
        selection
    }
}

impl ExportDataset {
    pub fn for_job_request(
        store: &Store,
        job: &str,
        request: &ExportJobRequest,
    ) -> ReportResult<Self> {
        frozen::job::capture(store, job, request)
    }

    pub fn resume_job_request(
        store: &Store,
        job: &str,
        request: &ExportJobRequest,
    ) -> ReportResult<Self> {
        frozen::job::resume(store, job, request)
    }

    pub fn recover_job_request(
        store: &Store,
        job: &str,
        request: &ExportJobRequest,
    ) -> ReportResult<Self> {
        frozen::job::recover(store, job, request)
    }
}
