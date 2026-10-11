mod common;

use census_domain::model::{
    CanonicalAthlete, CoverageRow, Gender, GradYear, RetainedConflict, ReviewCase, ReviewState,
    SourceEntityKind, SourceIdentity, SourceNamespace, SourceObjectIdentity,
    CANONICAL_ID_COLLISION_FAMILY, COHORT_UNVERIFIED_FAMILY,
};
use census_domain::UsJurisdiction;
use census_reconcile::index::derive;
use census_store::{Store, Table};
use common::{athlete_in, school_in, TestResult};

#[test]
fn same_name_across_schools_never_merges() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let west = school_in(&store, UsJurisdiction::Wisconsin, "Madison West")?;
    let east = school_in(&store, UsJurisdiction::Minnesota, "Ada Borup")?;
    athlete_in(
        &store,
        &west,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        "smith-west-1",
    )?;
    athlete_in(
        &store,
        &east,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        "smith-east-1",
    )?;

    let report = derive(&store, "index", "2026-09-22", common::season()?)?;
    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    if athletes.len() != 2 {
        return Err(format!("athlete rows: {}", athletes.len()).into());
    }
    let identities: Vec<SourceObjectIdentity> = store.scan(Table::SourceIdentities)?;
    let athlete_rows: Vec<_> = identities
        .iter()
        .filter(|row| row.entity == SourceEntityKind::Athletes)
        .collect();
    if athlete_rows.len() != 2 {
        return Err(format!("athlete identity rows: {}", athlete_rows.len()).into());
    }
    if athlete_rows[0].canonical_id == athlete_rows[1].canonical_id {
        return Err("a name at two schools is two identities".into());
    }
    let conflicts: Vec<RetainedConflict> = store.scan(Table::Conflicts)?;
    let collision_free = conflicts
        .iter()
        .all(|row| row.family != CANONICAL_ID_COLLISION_FAMILY);
    if !collision_free {
        return Err("a shared name is not a shared identity".into());
    }
    if report.source_identities != identities.len() {
        return Err(format!("source identities: {}", report.source_identities).into());
    }
    Ok(())
}

#[test]
fn same_name_same_school_splits_on_cohort() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = school_in(&store, UsJurisdiction::Wisconsin, "Madison West")?;
    athlete_in(
        &store,
        &school,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        "smith-2027",
    )?;
    athlete_in(
        &store,
        &school,
        "Jordan Smith",
        GradYear::new(2028).ok_or("2028 is a class")?,
        Gender::Boys,
        "smith-2028",
    )?;

    derive(&store, "index", "2026-09-22", common::season()?)?;
    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    if athletes.len() != 2 {
        return Err(format!("athlete rows: {}", athletes.len()).into());
    }
    let identities: Vec<SourceObjectIdentity> = store.scan(Table::SourceIdentities)?;
    let athlete_rows = identities
        .iter()
        .filter(|row| row.entity == SourceEntityKind::Athletes)
        .count();
    if athlete_rows != 2 {
        return Err(format!("athlete identity rows: {athlete_rows}").into());
    }
    Ok(())
}

#[test]
fn canonical_id_collision_is_retained() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = school_in(&store, UsJurisdiction::Wisconsin, "Abbotsford")?;
    let jane = athlete_in(
        &store,
        &school,
        "Jane Doe",
        GradYear::CO2027,
        Gender::Girls,
        "doe-7",
    )?;
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "reyes-9");
    let mut marta = CanonicalAthlete::new(
        &school,
        "Marta Reyes",
        GradYear::CO2027,
        Gender::Girls,
        source,
    );
    marta.id = jane.id.clone();
    store.append(Table::Athletes, &marta)?;

    let report = derive(&store, "index", "2026-09-22", common::season()?)?;
    let conflicts: Vec<RetainedConflict> = store.scan(Table::Conflicts)?;
    let collision = conflicts
        .iter()
        .find(|row| row.family == CANONICAL_ID_COLLISION_FAMILY)
        .ok_or("the collision reaches the queue")?;
    if collision.subject_id != jane.id.as_str() {
        return Err(format!("collision subject: {}", collision.subject_id).into());
    }
    let detail = collision.detail.to_lowercase();
    if !detail.contains("jane") {
        return Err(detail.clone().into());
    }
    if !detail.contains("marta") {
        return Err(detail.into());
    }
    if report.conflicts == 0 {
        return Err("the collision reaches the report".into());
    }
    Ok(())
}

#[test]
fn second_provider_identity_does_not_double_count() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = school_in(&store, UsJurisdiction::Wisconsin, "Abbotsford")?;
    let mut athlete = CanonicalAthlete::new(
        &school,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "smith-1"),
    );
    athlete.add_identity(SourceIdentity::new(
        SourceNamespace::AthleticNet {
            kind: "athlete".to_string(),
        },
        "987654",
    ));
    store.append(Table::Athletes, &athlete)?;

    derive(&store, "index", "2026-09-22", common::season()?)?;
    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    if athletes.len() != 1 {
        return Err(format!("athlete rows: {}", athletes.len()).into());
    }
    let identities: Vec<SourceObjectIdentity> = store.scan(Table::SourceIdentities)?;
    if identities.len() != 2 {
        return Err(format!("identity rows: {}", identities.len()).into());
    }
    let conflicts: Vec<RetainedConflict> = store.scan(Table::Conflicts)?;
    let collision_free = conflicts
        .iter()
        .all(|row| row.family != CANONICAL_ID_COLLISION_FAMILY);
    if !collision_free {
        return Err("one athlete with two provider ids is not a collision".into());
    }
    Ok(())
}

#[test]
fn cohort_membership_is_not_identity() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = school_in(&store, UsJurisdiction::Wisconsin, "Abbotsford")?;
    let athlete = athlete_in(
        &store,
        &school,
        "Jane Doe",
        GradYear::CO2027,
        Gender::Girls,
        "doe-7",
    )?;

    derive(&store, "index", "2026-09-22", common::season()?)?;
    let cases: Vec<ReviewCase> = store.scan(Table::ReviewCases)?;
    let unverified = cases
        .iter()
        .find(|case| case.family == COHORT_UNVERIFIED_FAMILY)
        .ok_or("the cohort finding is retained")?;
    if unverified.subject_id != athlete.id.as_str() {
        return Err(format!("cohort subject: {}", unverified.subject_id).into());
    }
    if unverified.state != ReviewState::Retained {
        return Err(format!("cohort finding state: {:?}", unverified.state).into());
    }
    let coverage: Vec<CoverageRow> = store.scan(Table::Coverage)?;
    let wisconsin = coverage
        .iter()
        .find(|row| row.id == "jurisdiction:WI")
        .ok_or("a Wisconsin denominator")?;
    let cohort_athletes = wisconsin.metrics.get("cohort_athletes");
    if cohort_athletes != Some(&1) {
        return Err(format!("cohort athletes: {cohort_athletes:?}").into());
    }
    let jurisdictions = coverage
        .iter()
        .filter(|row| row.id.starts_with("jurisdiction:"))
        .count();
    let expected = UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1);
    if jurisdictions != expected {
        return Err(format!("jurisdiction rows: {jurisdictions} expected {expected}").into());
    }
    Ok(())
}
