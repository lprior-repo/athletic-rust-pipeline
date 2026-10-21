use super::{fixtures, graph, protocol, retained_case, ReviewCase, ReviewState, Table, TestResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Gender, GradYear, RetainedConflict, SourceIdentity,
    SourceNamespace,
};
use census_domain::public_identity_history::PublicResolution;
use census_domain::UsJurisdiction;
use std::sync::Arc;

#[tokio::test]
async fn standing_conflict_and_retained_case_survive_compatible_advice() -> TestResult {
    let controlled = protocol::controlled()?;
    let store = Arc::new(controlled.store().clone());
    let rows = fixtures::seed(&store)?;
    let mut old = retained_case("original-standing-case");
    old.member_ids = rows.iter().map(|row| row.id.cast()).collect();
    let conflict = RetainedConflict::new(
        "Athlete identity",
        &old.subject_id,
        &old.subject,
        "standing conflicting source fact",
    );
    store.replace(Table::ReviewCases, &old)?;
    store.replace(Table::Conflicts, &conflict)?;
    let before = graph::read(&store)?;
    protocol::exercise(protocol::journal(controlled)?, protocol::Boundary::Workbook).await?;
    graph::unchanged(&store, &before, &rows)?;
    check!(store.scan::<ReviewCase>(Table::ReviewCases)?.contains(&old));
    check!(store
        .scan::<RetainedConflict>(Table::Conflicts)?
        .contains(&conflict));
    check!(store
        .journal_payload("identity_retained_supersession_v1", &old.id)?
        .is_none());
    graph::legacy_write_refused(&store)
}

#[tokio::test]
async fn opposite_categories_remain_unknown_without_graph_authority() -> TestResult {
    let controlled = protocol::controlled()?;
    let store = Arc::new(controlled.store().clone());
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Superior High",
        "superior high",
        Some("Superior"),
    );
    let shared = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "551235")
        .with_url("https://timer.test/athletes/551235");
    let rows = vec![
        CanonicalAthlete::new(
            &school.id,
            "Alex Morgan",
            GradYear::CO2027,
            Gender::Boys,
            shared.clone(),
        )?,
        CanonicalAthlete::new(
            &school.id,
            "Alex Morgan",
            GradYear::CO2027,
            Gender::Girls,
            shared,
        )?,
    ];
    store.append(Table::Schools, &school)?;
    store.append_many(Table::Athletes, &rows)?;
    let before = graph::read(&store)?;
    protocol::exercise(protocol::journal(controlled)?, protocol::Boundary::Workbook).await?;
    graph::unchanged(&store, &before, &rows)?;
    for row in &rows {
        check!(eq; graph::read(&store)?.resolve_candidate(&row.id.cast())?, PublicResolution::Unknown);
    }
    check!(store
        .scan::<ReviewCase>(Table::ReviewCases)?
        .iter()
        .any(|case| {
            case.member_ids
                .iter()
                .any(|member| rows.iter().any(|row| member.as_str() == row.id.as_str()))
                && case.state != ReviewState::Resolved
        }));
    graph::legacy_write_refused(&store)
}

#[tokio::test]
async fn compatible_unknown_category_keeps_historical_retained_case_in_both_orders() -> TestResult {
    for reverse in [false, true] {
        compatible(reverse).await?;
    }
    Ok(())
}

async fn compatible(reverse: bool) -> TestResult {
    let controlled = protocol::controlled()?;
    let store = Arc::new(controlled.store().clone());
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Superior High",
        "superior high",
        Some("Superior"),
    );
    let source = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "551234")
        .with_url("https://timer.test/athletes/551234");
    let mut rows = vec![
        CanonicalAthlete::new(
            &school.id,
            "Alex Morgan",
            GradYear::CO2027,
            Gender::Girls,
            source.clone(),
        )?,
        CanonicalAthlete::new(
            &school.id,
            "Alex Morgan",
            GradYear::CO2027,
            Gender::Unknown,
            source,
        )?,
    ];
    if reverse {
        rows.reverse();
    }
    store.append(Table::Schools, &school)?;
    store.append_many(Table::Athletes, &rows)?;
    let first = rows.first().ok_or("compatible subject absent")?;
    let mut old = retained_case(first.id.as_str());
    let (binding, digest) = old.id.rsplit_once(':').ok_or("case digest absent")?;
    let (subject, _) = binding.rsplit_once(":p").ok_or("case policy absent")?;
    old.id = format!("{subject}:p1:{digest}");
    old.member_ids = rows.iter().map(|row| row.id.cast()).collect();
    store.replace(Table::ReviewCases, &old)?;
    let before = graph::read(&store)?;
    protocol::exercise(protocol::journal(controlled)?, protocol::Boundary::Workbook).await?;
    graph::unchanged(&store, &before, &rows)?;
    check!(store.scan::<ReviewCase>(Table::ReviewCases)?.contains(&old));
    check!(store
        .journal_payload("identity_retained_supersession_v1", &old.id)?
        .is_none());
    graph::legacy_write_refused(&store)
}
