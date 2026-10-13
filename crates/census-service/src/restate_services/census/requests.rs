use std::path::PathBuf;

use crate::census::seal::{JournalCounts, SealRequest as StoreSealRequest};
use census_domain::model::CensusRun;
use census_domain::UsJurisdiction;
use census_reconcile::identity::admitted_scope;
use census_report::report::Scope;
use restate_sdk::prelude::{HandlerError, TerminalError};

use super::super::wire::{OpenWorkReply, OpenWorkRequest, SealRequest};

pub(super) fn admitted_jurisdictions(
    requested: &[UsJurisdiction],
) -> Result<Vec<UsJurisdiction>, HandlerError> {
    let admitted = admitted_scope(requested);
    for jurisdiction in &admitted {
        if let Err(outside) = jurisdiction.require_census_scope() {
            return Err(TerminalError::new(outside.to_string()).into());
        }
    }
    Ok(UsJurisdiction::CENSUS_SCOPE
        .iter()
        .copied()
        .filter(|state| admitted.contains(state))
        .collect())
}

pub(super) fn run_request(request: &SealRequest) -> OpenWorkRequest {
    OpenWorkRequest {
        season: request.season,
        revision: request.revision,
        source_objects: request.source_objects.clone(),
    }
}

pub(super) fn store_request(
    request: &SealRequest,
    journal: &OpenWorkReply,
    run: CensusRun,
    jurisdictions: Vec<UsJurisdiction>,
) -> StoreSealRequest {
    StoreSealRequest {
        grad_year: request.grad_year,
        scope: if request.all_sources {
            Scope::AllSources
        } else {
            Scope::Core
        },
        jurisdictions,
        workbook: request.workbook.as_ref().map(PathBuf::from),
        write: request.write,
        journal: Some(JournalCounts {
            jurisdiction_sweeps: journal.jurisdiction_sweeps,
            source_objects: journal.source_objects,
            silent_sources: journal.silent_sources.clone(),
        }),
        source_failures: None,
        run: Some(run),
    }
}
