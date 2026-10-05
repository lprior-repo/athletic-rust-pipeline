use super::{athlete, cases, school, store_of, verdicts, TestResult};
use crate::{reconcile_athletes, RULE_REVIEWER};
use census_domain::model::{
    Gender, IdentityApplication, IdentityProjectionBuilder, IdentityStatus, ReviewCase,
    ReviewState, ReviewVerdictRecord, SourceIdentity, SourceNamespace,
};
use census_store::Store;

pub(super) fn admits_one_identity(
    store: &Store,
    filed: &[ReviewCase],
    recorded: &[ReviewVerdictRecord],
    case: &ReviewCase,
) -> TestResult {
    let index = store.athlete_identity_index()?;
    let builder = IdentityProjectionBuilder::new(index, filed, recorded)?;
    let mut applications = Vec::new();
    for (_, application) in builder.reviewed_applications("2026-09-23") {
        match application? {
            IdentityApplication::Accepted(accepted) => applications.push(accepted),
            IdentityApplication::Retained(_) => {}
        }
    }
    check!(eq; applications.len(), 1, "the generated case must pass actual identity admission");
    store.apply_identity_decisions(&applications)?;
    let projection = store.athlete_identity_projection()?;
    let roots: std::collections::BTreeSet<_> = case
        .member_ids
        .iter()
        .map(|member| projection.canonical_id(member.as_str()))
        .collect();
    check!(eq; roots.len(), 1);
    for member in &case.member_ids {
        check!(eq; projection.status(member.as_str())?, IdentityStatus::Verified);
    }
    Ok(())
}

#[test]
fn a_link_retained_with_a_distinct_document_decides_one_athlete() -> TestResult {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let mut linked = athlete(&west.id, "Jordan Smith", Gender::Boys, "14399169");
    linked.add_identity(
        SourceIdentity::new(SourceNamespace::TfrrsAthlete, "991114")
            .with_url("https://wi.milesplit.com/teams/123/roster"),
    );
    let mut owner = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    owner.source = Some(
        SourceIdentity::new(SourceNamespace::TfrrsAthlete, "991114")
            .with_url("https://www.tfrrs.org/athletes/991114.html"),
    );
    let (_dir, store) = store_of(&[lakeland, west], &[linked, owner])?;
    let report = reconcile_athletes(&store, "2026-09-23", false)?;
    check!(eq; report.decided, 1, "the corroboration rule decides one finding");
    check!(eq; report.pending, 0, "nothing is left open");
    let filed = cases(&store)?;
    let case = filed.first().ok_or("one case")?;
    check!(eq; case.state, ReviewState::Resolved, "a decision closes it");
    let recorded = verdicts(&store)?;
    let verdict = recorded.first().ok_or("one verdict")?;
    check!(eq; verdict.reviewer, RULE_REVIEWER);
    admits_one_identity(&store, &filed, &recorded, case)?;
    Ok(())
}

#[test]
fn a_link_without_a_retained_document_stays_pending() -> TestResult {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let mut linked = athlete(&west.id, "Jordan Smith", Gender::Boys, "14399169");
    linked.add_identity(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "991114"));
    let mut owner = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    owner.source = Some(
        SourceIdentity::new(SourceNamespace::TfrrsAthlete, "991114")
            .with_url("https://www.tfrrs.org/athletes/991114.html"),
    );
    let (_dir, store) = store_of(&[lakeland, west], &[linked, owner])?;
    let report = reconcile_athletes(&store, "2026-09-23", false)?;
    check!(eq; report.decided, 0, "an unattested link decides nothing");
    check!(eq; report.pending, 1, "the finding stays open for advice");
    Ok(())
}
