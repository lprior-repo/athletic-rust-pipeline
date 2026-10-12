use std::collections::{BTreeMap, BTreeSet};

use census_domain::model::{
    AppliedAthleteIdentity, CanonicalAthlete, CanonicalSchool, IdentityApplication,
    IdentityProjectionBuilder, IdentityStatus, ReviewCase, ReviewState, ReviewVerdictRecord,
    SchoolId,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde::Deserialize;

use crate::consensus::tests::support::TestResult;
use crate::reconcile_athletes;

#[derive(Deserialize)]
struct Frozen {
    data: Measured,
}

#[derive(Deserialize)]
struct Measured {
    athletes: Vec<CanonicalAthlete>,
    schools: BTreeMap<SchoolId, School>,
    #[serde(default)]
    review_cases: Vec<ReviewCase>,
    #[serde(default)]
    verdicts: Vec<ReviewVerdictRecord>,
    #[serde(default)]
    identity_decisions: Vec<AppliedAthleteIdentity>,
}

#[derive(Deserialize)]
struct School {
    name: String,
    state: Option<UsJurisdiction>,
}

fn measured_pair() -> TestResult<Measured> {
    Ok(serde_json::from_str::<Frozen>(include_str!(
        "athlete_clusters_tests/o8rm/nyah_bryant.json"
    ))?
    .data)
}

fn seed(measured: &Measured, rows: &[CanonicalAthlete]) -> TestResult<(tempfile::TempDir, Store)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let ids: BTreeSet<_> = rows.iter().map(|row| &row.school).collect();
    for id in ids {
        let school = measured
            .schools
            .get(id)
            .ok_or("measured school is present")?;
        let (mut canonical, _) = CanonicalSchool::new(
            school.state.ok_or("measured school has a state")?,
            &school.name,
            school.name.to_ascii_lowercase(),
            None,
        );
        canonical.id = id.clone();
        store.append(Table::Schools, &canonical)?;
    }
    store.append_many(Table::Athletes, rows)?;
    Ok((dir, store))
}

fn apply_review(store: &Store, expected: (usize, usize)) -> TestResult {
    let report = reconcile_athletes(store, "2026-10-12", false)?;
    check!(eq; (report.decided, report.pending), expected);
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    let verdicts = store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
    let builder =
        IdentityProjectionBuilder::new(store.athlete_identity_index()?, &cases, &verdicts)?;
    let mut accepted = Vec::new();
    for (_, application) in builder.reviewed_applications("2026-10-12") {
        match application? {
            IdentityApplication::Accepted(decision) => accepted.push(decision),
            IdentityApplication::Retained(issue) => {
                return Err(format!("generated decision was refused: {issue:?}").into());
            }
        }
    }
    check!(eq; accepted.len(), expected.0);
    store.apply_identity_decisions(&accepted)?;
    println!("cases={}", serde_json::to_string(&cases)?);
    println!("verdicts={}", serde_json::to_string(&verdicts)?);
    println!(
        "applications={}",
        serde_json::to_string(
            &store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?
        )?
    );
    Ok(())
}

fn retained_rows(store: &Store, rows: &[CanonicalAthlete]) -> TestResult {
    let mut stored = store.scan::<CanonicalAthlete>(Table::Athletes)?;
    let mut expected = rows.to_vec();
    stored.sort_by(|left, right| left.id.cmp(&right.id));
    expected.sort_by(|left, right| left.id.cmp(&right.id));
    check!(eq; stored, expected, "every source record and evidence field remains intact");
    Ok(())
}

#[test]
fn o8rm_measured_shared_primary_unknown_gender_is_merged_without_losing_observations() -> TestResult
{
    let measured = measured_pair()?;
    let mut rows = measured.athletes.clone();
    check!(eq; rows.len(), 2);
    check!(eq; rows.iter().map(|row| row.evidence.len()).sum::<usize>(), 3);
    let mut decisions = Vec::new();
    for _ in 0..2 {
        let (_dir, store) = seed(&measured, &rows)?;
        let before = store.athlete_identity_projection()?;
        check!(eq; before.canonical_id("ath_subject_f8dad00a59e24d7f"), "ath_subject_f8dad00a59e24d7f");
        apply_review(&store, (1, 0))?;
        let projection = store.athlete_identity_projection()?;
        for row in &rows {
            check!(eq; projection.canonical_id(row.id.as_str()), "ath_subject_5bfe83d3634b1784");
            check!(eq; projection.status(row.id.as_str())?, IdentityStatus::Verified);
        }
        retained_rows(&store, &rows)?;
        let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
        check!(eq; cases.first().map(|case| case.state), Some(ReviewState::Resolved));
        decisions.push(store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?);
        rows.reverse();
    }
    check!(eq; decisions.first(), decisions.last(), "input order cannot alter the accepted identity");
    Ok(())
}

#[path = "athlete_cluster_findings_tests/genders.rs"]
mod genders;

#[path = "athlete_cluster_findings_tests/groups.rs"]
mod groups;
