use crate::consensus::tests::support::TestResult;
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Gender, GradYear, Grade, ObservedGrade, ReviewCase,
    ReviewState, ReviewVerdictRecord, SchoolId, SchoolYear, SourceIdentity, SourceNamespace,
    SourceRef, ATHLETE_IDENTITY_FAMILY,
};
use census_store::{Store, Table};

use super::{reconcile_athletes, RULE_REVIEWER};

fn school(name: &str) -> CanonicalSchool {
    let slug = name.to_ascii_lowercase().replace(' ', "-");
    let (school, _) = CanonicalSchool::new(census_domain::UsJurisdiction::Wisconsin, name, &slug);
    school
}

fn athlete(school: &SchoolId, name: &str, gender: Gender, provider_id: &str) -> CanonicalAthlete {
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, provider_id);
    let mut row = CanonicalAthlete::new(school, name, GradYear::CO2027, gender, source);
    row.evidence.push(census_domain::model::Evidence::parsed(
        SourceRef::new(
            "captured_roster",
            Some("https://wi.milesplit.com/teams/123/roster".into()),
        ),
        "2026-09-23",
    ));
    row
}

fn grade(row: &mut CanonicalAthlete, value: u8, year: i16) -> TestResult {
    row.observed_grades.push(ObservedGrade {
        grade: Grade::new(value).ok_or("a valid grade")?,
        school_year: SchoolYear::new(year).ok_or("a valid school year")?,
        source: SourceRef::new("milesplit_roster", None),
    });
    Ok(())
}

fn store_of(
    schools: &[CanonicalSchool],
    rows: &[CanonicalAthlete],
) -> TestResult<(tempfile::TempDir, Store)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    for school in schools {
        store.append(Table::Schools, school)?;
    }
    for row in rows {
        store.append(Table::Athletes, row)?;
    }
    Ok((dir, store))
}

fn cases(store: &Store) -> TestResult<Vec<ReviewCase>> {
    Ok(store.scan::<ReviewCase>(Table::ReviewCases)?)
}

fn verdicts(store: &Store) -> TestResult<Vec<ReviewVerdictRecord>> {
    Ok(store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?)
}

#[test]
fn a_provider_object_on_two_schools_is_one_athlete() -> TestResult {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let first = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    let second = athlete(&west.id, "Jordan Smith", Gender::Boys, "14399169");
    let (_dir, store) = store_of(&[lakeland, west], &[first, second])?;
    let report = reconcile_athletes(&store, "2026-09-23", false)?;
    check!(eq; report.decided, 1, "the agreement rule decides one finding");
    check!(eq; report.pending, 0, "nothing is left open");
    let filed = cases(&store)?;
    let case = filed.first().ok_or("one case")?;
    check!(eq; case.family, ATHLETE_IDENTITY_FAMILY);
    check!(eq; case.state, ReviewState::Resolved, "a decision closes it");
    check!(
        case.detail.contains("Lakeland") && case.detail.contains("Madison West"),
        "the case names both sites: {}",
        case.detail
    );
    let recorded = verdicts(&store)?;
    let verdict = recorded.first().ok_or("one verdict")?;
    check!(eq; verdict.reviewer, RULE_REVIEWER);
    let index = store.athlete_identity_index()?;
    let builder = census_domain::model::IdentityProjectionBuilder::new(index, &filed, &recorded)?;
    let mut applications = Vec::new();
    for (_, application) in builder.reviewed_applications("2026-09-23") {
        match application? {
            census_domain::model::IdentityApplication::Accepted(accepted) => {
                applications.push(accepted)
            }
            census_domain::model::IdentityApplication::Retained(_) => {}
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
        check!(eq; projection.status(member.as_str())?, census_domain::model::IdentityStatus::Verified);
    }
    Ok(())
}

#[test]
fn the_deterministic_lane_does_not_merge_contradictory_rows() -> TestResult {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let mut boys = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    let mut girls = athlete(&west.id, "Jordan Smith", Gender::Boys, "14399169");
    grade(&mut boys, 11, 2025)?;
    grade(&mut girls, 10, 2025)?;
    let (_dir, store) = store_of(&[lakeland, west], &[boys, girls])?;
    let report = reconcile_athletes(&store, "2026-09-23", false)?;
    check!(eq; report.pending, 1);
    check!(eq; report.decided, 0);
    check!(eq; cases(&store)?.first().map(|case| case.state), Some(ReviewState::Pending));
    check!(
        verdicts(&store)?.is_empty(),
        "an undecided finding carries no verdict"
    );
    Ok(())
}

#[test]
fn the_deterministic_lane_does_not_merge_gender_contradictory_rows() -> TestResult {
    let lakeland = school("Lakeland");
    let boys = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    let girls = athlete(&lakeland.id, "Jordan Smith", Gender::Girls, "14399169");
    let (_dir, store) = store_of(&[lakeland], &[boys, girls])?;
    let report = reconcile_athletes(&store, "2026-09-23", false)?;
    check!(eq; report.pending, 1);
    check!(eq; report.decided, 0);
    check!(eq; cases(&store)?.first().map(|case| case.state), Some(ReviewState::Pending));
    check!(verdicts(&store)?.is_empty());
    Ok(())
}

#[test]
fn a_row_naming_two_objects_of_one_provider_is_filed() -> TestResult {
    let lakeland = school("Lakeland");
    let mut row = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    row.add_identity(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "14407777",
    ));
    let (_dir, store) = store_of(&[lakeland], &[row])?;
    let report = reconcile_athletes(&store, "2026-09-23", false)?;
    check!(eq; report.aliases, 1, "one row carries two objects");
    check!(eq; report.pending, 1);
    let filed = cases(&store)?;
    let case = filed.first().ok_or("one case")?;
    check!(eq; case.state, ReviewState::Pending);
    check!(
        case.detail.contains("14399169") && case.detail.contains("14407777"),
        "the case names both objects: {}",
        case.detail
    );
    check!(verdicts(&store)?.is_empty());
    Ok(())
}

#[test]
fn a_second_pass_leaves_a_decided_case_alone() -> TestResult {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let first = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    let second = athlete(&west.id, "Jordan Smith", Gender::Boys, "14399169");
    let (_dir, store) = store_of(&[lakeland, west], &[first, second])?;
    reconcile_athletes(&store, "2026-09-23", false)?;
    let decided = cases(&store)?;
    let verdict = verdicts(&store)?;
    let again = reconcile_athletes(&store, "2026-09-24", false)?;
    check!(eq; again.filed, 0, "nothing new is filed");
    check!(eq; again.held, 1, "the standing decision is reported, not rewritten");
    check!(eq; cases(&store)?, decided, "the case row is untouched");
    check!(eq; verdicts(&store)?, verdict, "the verdict is untouched");
    Ok(())
}

#[test]
fn the_case_does_not_depend_on_append_order() -> TestResult {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let first = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    let second = athlete(&west.id, "Jordan Smith", Gender::Boys, "14399169");
    let (_dir, forwards) = store_of(
        &[lakeland.clone(), west.clone()],
        &[first.clone(), second.clone()],
    )?;
    let (_other, backwards) = store_of(&[west, lakeland], &[second, first])?;
    reconcile_athletes(&forwards, "2026-09-23", false)?;
    reconcile_athletes(&backwards, "2026-09-23", false)?;
    let ids = |store: &Store| -> TestResult<Vec<String>> {
        Ok(cases(store)?.into_iter().map(|c| c.id).collect())
    };
    check!(eq; ids(&forwards)?, ids(&backwards)?, "one finding, one case id");
    Ok(())
}

#[test]
fn a_later_pass_on_the_same_date_applies_its_own_payload() -> TestResult {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let first = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    let second = athlete(&west.id, "Jordan Smith", Gender::Boys, "14399169");
    let (_dir, store) = store_of(&[lakeland, west], &[first, second])?;
    let morning = reconcile_athletes(&store, "2026-09-25", false)?;
    check!(eq; morning.decided, 1, "the first pair is decided");
    let standing = verdicts(&store)?;
    let east = school("East");
    let north = school("North");
    let third = athlete(&east.id, "Riley Chen", Gender::Boys, "22001144");
    let fourth = athlete(&north.id, "Riley Chen", Gender::Boys, "22001144");
    for school_row in [east, north] {
        store.append(Table::Schools, &school_row)?;
    }
    for row in [third, fourth] {
        store.append(Table::Athletes, &row)?;
    }
    let afternoon = reconcile_athletes(&store, "2026-09-25", false)?;
    check!(eq; afternoon.decided, 1, "the new pair is decided");
    let all = verdicts(&store)?;
    check!(eq; all.len(), 2, "one verdict per decided pair");
    check!(
        standing.iter().all(|verdict| all.contains(verdict)),
        "the first pass's verdict still stands: {all:?}"
    );
    let resolved = cases(&store)?
        .into_iter()
        .filter(|case| case.state == ReviewState::Resolved)
        .count();
    check!(eq; resolved, 2, "both decided cases are resolved in the store");
    Ok(())
}

#[test]
fn unsupported_grade_observation_blocks_shared_provider_acceptance() -> TestResult {
    let lakeland = school("Lakeland");
    let west = school("Madison West");
    let mut first = athlete(&lakeland.id, "Jordan Smith", Gender::Boys, "14399169");
    let mut second = athlete(&west.id, "Jordan Smith", Gender::Boys, "14399169");
    grade(&mut first, 11, 2025)?;
    grade(&mut second, 11, 2025)?;
    first.observed_grades.push(ObservedGrade {
        grade: Grade::new(9).ok_or("supported grade")?,
        school_year: SchoolYear::new(2040).ok_or("supported school year")?,
        source: SourceRef::new(
            "milesplit_roster",
            Some("https://fixture.example/unsupported-grade".to_string()),
        ),
    });
    let (_dir, store) = store_of(&[lakeland, west], &[first, second])?;
    let report = reconcile_athletes(&store, "2026-09-30", false)?;
    check!(eq; (report.pending, report.decided), (1, 0));
    check!(verdicts(&store)?.is_empty());
    let filed = cases(&store)?;
    let case = filed.first().ok_or("one case")?;
    check!(eq; case.state, ReviewState::Pending);
    check!(case.detail.contains("2040"));
    check!(case
        .detail
        .contains("https://fixture.example/unsupported-grade"));
    Ok(())
}

#[path = "athlete_clusters_tests/three_members.rs"]
mod three_members;

#[path = "athlete_clusters_tests/reopened_receipt.rs"]
mod reopened_receipt;
