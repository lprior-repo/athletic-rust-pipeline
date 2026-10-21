use super::{fixtures, graph, protocol, Journal, Table, TestResult};
use census_domain::model::{ProfileSelectionV1, SchoolYear, StoreIdentity};
use census_domain::UsJurisdiction;
use census_report::export::ExportJobRequest;
use std::sync::Arc;

#[derive(Clone, Copy)]
enum Mismatch {
    ForeignStore,
    Season,
    Jurisdictions,
}

#[tokio::test]
async fn selected_scope_refusals_precede_review_writes() -> TestResult {
    for mismatch in [
        Mismatch::ForeignStore,
        Mismatch::Season,
        Mismatch::Jurisdictions,
    ] {
        refuses(mismatch).await?;
    }
    Ok(())
}

async fn refuses(mismatch: Mismatch) -> TestResult {
    let controlled = protocol::controlled()?;
    let store = Arc::new(controlled.store().clone());
    let rows = fixtures::seed(&store)?;
    let original = protocol::journal(controlled)?;
    let mut request = original.original.clone();
    match mismatch {
        Mismatch::ForeignStore => {
            let selection = request.export_job.require_selected_publication();
            request.export_job = ExportJobRequest::SelectedPublicationV1 {
                selection: ProfileSelectionV1::new(
                    StoreIdentity::parse(&"0".repeat(64))?,
                    selection.profile_id().clone(),
                ),
            };
        }
        Mismatch::Season => request.season = SchoolYear::new(2025).ok_or("fixture season")?,
        Mismatch::Jurisdictions => request.jurisdictions = vec![UsJurisdiction::Colorado],
    }
    let mut journal = Journal::new(&request)?;
    journal.selected_workbook = original.selected_workbook;
    let decisions = store.scan::<super::AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    let state = graph::state(&store)?;
    let accepted = graph::read(&store)?;
    let journal = protocol::exercise(journal, protocol::Boundary::Refusal).await?;
    check!(journal.terminal_failure.is_some());
    check!(super::deterministic::run_names(&journal)?.is_empty());
    check!(!journal.calls.iter().any(|call| {
        call.text(1).is_ok_and(|service| {
            matches!(
                service.as_str(),
                "JurisdictionCensus" | "TeamsSource" | "Workbook"
            )
        }) || call.text(2).is_ok_and(|handler| handler == "bind_run")
    }));
    check!(eq; graph::state(&store)?, state);
    graph::unchanged(&store, &accepted, &rows)?;
    check!(eq; store.scan::<super::AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?, decisions);
    Ok(())
}
