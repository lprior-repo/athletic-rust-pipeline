use super::{fixtures, graph, protocol, TestResult};
use census_domain::model::*;
use census_domain::public_identity_history::PublicResolution;
use census_store::Table;
use std::sync::Arc;

mod result;

#[tokio::test]
async fn national_keeps_transfer_candidates_and_original_result_owned_after_cold_reopen(
) -> TestResult {
    let controlled = protocol::controlled()?;
    let store = Arc::new(controlled.store().clone());
    let rows = fixtures::seed_transfer(&store)?;
    let first = rows.first().ok_or("missing original owner")?;
    let transferred = rows.last().ok_or("missing transferred owner")?;
    check!(ne; first.school, transferred.school);
    let performance = result::historical_performance(&store, first)?;
    let before = graph::read(&store)?;
    let mut journal =
        protocol::exercise(protocol::journal(controlled)?, protocol::Boundary::Workbook).await?;
    graph::unchanged(&store, &before, &rows)?;
    check!(store
        .scan::<CanonicalPerformance>(Table::Performances)?
        .contains(&performance));
    for row in &rows {
        check!(eq; graph::read(&store)?.resolve_candidate(&row.id.cast())?, PublicResolution::Unknown);
    }
    let expected_rows = graph::state(&store)?;
    drop(store);
    let controlled = journal
        .selected_workbook
        .take()
        .ok_or("controlled source absent")?;
    let reopened = crate::restate_services::jobs::selected_fixture::reopen(controlled)
        .map_err(|error| -> Box<dyn std::error::Error> { error })?;
    graph::unchanged(reopened.store(), &before, &rows)?;
    check!(eq; graph::state(reopened.store())?, expected_rows);
    check!(reopened
        .store()
        .scan::<CanonicalPerformance>(Table::Performances)?
        .contains(&performance));
    let retained = reopened.store().scan::<CanonicalAthlete>(Table::Athletes)?;
    for row in &rows {
        check!(retained.contains(row));
    }
    graph::legacy_write_refused(reopened.store())
}
