use anyhow::Result;
use census_crawl::ingress::Ingress;
use census_domain::{
    model::{Revision, SchoolYear},
    UsJurisdiction,
};
use census_reconcile::identity::{admitted_scope, WorkflowIdentity};
use census_service::restate_services::{HistoryWindow, JurisdictionCensusIngressClient};
use restate_sdk::prelude::*;
use std::num::NonZeroU32;

mod binding;
pub(super) use binding::{
    bound_request, refuse_on_caller_authority, retained_request, validate_selected,
};

#[derive(Debug, thiserror::Error)]
enum ResumeHistoryError {
    #[error("no jurisdiction has bound history for this run; resume cannot change its immutable history scope")]
    Unbound,
    #[error(
        "{jurisdiction} bound history {actual:?} disagrees with the run's history {expected:?}"
    )]
    Conflicting {
        jurisdiction: UsJurisdiction,
        expected: HistoryWindow,
        actual: HistoryWindow,
    },
}

pub(super) fn identity(
    season: SchoolYear,
    revision: Revision,
    jurisdictions: &[UsJurisdiction],
    ordinal: Option<NonZeroU32>,
) -> WorkflowIdentity {
    match ordinal {
        Some(ordinal) => {
            WorkflowIdentity::national_resume(season, revision, jurisdictions, ordinal)
        }
        None => WorkflowIdentity::national(season, revision, jurisdictions),
    }
}

pub(super) async fn history(
    ingestion: &Ingress,
    season: SchoolYear,
    revision: Revision,
    jurisdictions: &[UsJurisdiction],
) -> Result<HistoryWindow> {
    let mut bound = None;
    for jurisdiction in admitted_scope(jurisdictions) {
        let key = WorkflowIdentity::jurisdiction(jurisdiction, season, revision);
        let object = JurisdictionCensusIngressClient::from_client(ingestion.clone(), key.as_str());
        let Json(state) = object
            .state()
            .call()
            .await
            .map_err(census_service::ingress::error)?
            .into_body()
            .map_err(census_service::ingress::error)?;
        if let Some(actual) = state.history_window {
            if let Some(expected) = bound {
                if actual != expected {
                    return Err(ResumeHistoryError::Conflicting {
                        jurisdiction,
                        expected,
                        actual,
                    }
                    .into());
                }
            } else {
                bound = Some(actual);
            }
        }
    }
    bound.ok_or_else(|| ResumeHistoryError::Unbound.into())
}
