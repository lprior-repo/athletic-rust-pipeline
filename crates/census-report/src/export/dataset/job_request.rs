use super::{frozen, ExportDataset};
use crate::report::ReportResult;
use census_store::Store;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "contract", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExportJobRequest {
    #[default]
    LegacyV2,
    SelectedPublicationV1 {
        selection: census_domain::model::ProfileSelectionV1,
    },
}

impl ExportJobRequest {
    pub fn require_selected_publication(
        &self,
    ) -> ReportResult<&census_domain::model::ProfileSelectionV1> {
        match self {
            Self::SelectedPublicationV1 { selection } => Ok(selection),
            Self::LegacyV2 => Err(crate::report::ReportError::Invariant {
                detail: "current publication requires an explicit selected profile contract".into(),
            }),
        }
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
}
