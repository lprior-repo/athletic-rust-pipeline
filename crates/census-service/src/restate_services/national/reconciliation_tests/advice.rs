use super::{
    fixtures, graph, protocol, retained_case, ReviewCase, ReviewState, ReviewVerdictRecord, Table,
    TestResult,
};
use census_domain::public_identity_history::PublicResolution;
use std::sync::Arc;

#[tokio::test]
async fn shared_source_rules_are_advice_without_graph_establishment_or_merge() -> TestResult {
    let controlled = protocol::controlled()?;
    let store = Arc::new(controlled.store().clone());
    let athletes = fixtures::seed(&store)?;
    let unrelated = retained_case("unrelated");
    store.replace(Table::ReviewCases, &unrelated)?;
    let before = graph::read(&store)?;
    for athlete in &athletes {
        check!(eq; before.resolve_candidate(&athlete.id.cast())?, PublicResolution::Unknown);
    }
    protocol::exercise(protocol::journal(controlled)?, protocol::Boundary::Workbook).await?;
    graph::unchanged(&store, &before, &athletes)?;
    graph::legacy_write_refused(&store)?;
    let verdicts = store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
    check!(verdicts
        .iter()
        .any(|row| row.accepted && row.value == "same_person"));
    check!(store
        .scan::<ReviewCase>(Table::ReviewCases)?
        .contains(&unrelated));
    Ok(())
}

#[tokio::test]
async fn generic_review_advice_does_not_supersede_cases_or_issue_graph_identity() -> TestResult {
    let controlled = protocol::controlled()?;
    let store = Arc::new(controlled.store().clone());
    let athletes = fixtures::seed(&store)?;
    let generic: Vec<_> = athletes
        .iter()
        .map(|athlete| {
            ReviewCase::minted(
                "Athlete identity",
                athlete.id.as_str(),
                &athlete.canonical_name,
                "unresolved generic identity obligation",
            )
        })
        .collect();
    store.replace_many(Table::ReviewCases, &generic)?;
    let before = graph::read(&store)?;
    protocol::exercise(protocol::journal(controlled)?, protocol::Boundary::Workbook).await?;
    graph::unchanged(&store, &before, &athletes)?;
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    let verdicts = store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
    check!(verdicts
        .iter()
        .any(|row| row.accepted && row.value == "same_person"));
    for original in &generic {
        let current = cases
            .iter()
            .find(|case| case.id == original.id)
            .ok_or("generic case disappeared")?;
        check!(ne; current.state, ReviewState::Superseded);
        check!(store
            .journal_payload("identity_generic_supersession_v1", &original.id)?
            .is_none());
    }
    graph::legacy_write_refused(&store)
}
