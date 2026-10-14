mod common;

use census_domain::model::{
    CanonicalAthlete, Gender, GradYear, ReviewCase, ReviewState, ReviewVerdictRecord,
    SourceIdentity, SourceNamespace,
};
use census_review::{reconcile_athletes, RULE_REVIEWER};
use census_store::{Store, Table};
use common::{athlete_row, school_id, TestResult};

#[test]
fn deterministic_lane_decides_clean_transfer() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let west = school_id(&store, "Madison West")?;
    let east = school_id(&store, "Madison East")?;
    let first = athlete_row(&store, &west, "Jordan Smith", Gender::Boys, "1001", 11)?;
    let second = athlete_row(&store, &east, "Jordan Smith", Gender::Boys, "1001", 11)?;

    let report = reconcile_athletes(&store, "2026-09-22", false)?;
    if report.rows != 2 {
        return Err(format!("rows: {}", report.rows).into());
    }
    if report.objects != 1 {
        return Err(format!("objects: {}", report.objects).into());
    }
    if report.filed != 1 {
        return Err(format!("filed: {}", report.filed).into());
    }
    if report.decided != 1 {
        return Err(format!("decided: {}", report.decided).into());
    }
    if report.pending != 0 {
        return Err(format!("pending: {}", report.pending).into());
    }
    if report.aliases != 0 {
        return Err(format!("aliases: {}", report.aliases).into());
    }
    if report.held != 0 {
        return Err(format!("held: {}", report.held).into());
    }
    let cases: Vec<ReviewCase> = store.scan(Table::ReviewCases)?;
    if cases.len() != 1 {
        return Err(format!("review cases: {}", cases.len()).into());
    }
    if cases[0].state != ReviewState::Resolved {
        return Err(format!("case state: {:?}", cases[0].state).into());
    }
    let verdicts: Vec<ReviewVerdictRecord> = store.scan(Table::IdentityVerdicts)?;
    if verdicts.len() != 1 {
        return Err(format!("identity verdicts: {}", verdicts.len()).into());
    }
    if verdicts[0].case_id != cases[0].id {
        return Err(format!("verdict case: {:?}", verdicts[0].case_id).into());
    }
    if verdicts[0].reviewer != RULE_REVIEWER {
        return Err(format!("verdict reviewer: {:?}", verdicts[0].reviewer).into());
    }
    if verdicts[0].value != "same_person" {
        return Err(format!("verdict value: {:?}", verdicts[0].value).into());
    }
    if !verdicts[0].accepted {
        return Err("the verdict must be accepted".into());
    }
    if verdicts[0].confidence != 100 {
        return Err(format!("verdict confidence: {}", verdicts[0].confidence).into());
    }
    if verdicts[0].observed_at != "2026-09-22" {
        return Err(format!("verdict clock: {:?}", verdicts[0].observed_at).into());
    }
    let subject = verdicts[0].subject_id.as_str();
    if subject != first.id.as_str() && subject != second.id.as_str() {
        return Err("the verdict must name one of the two subjects".into());
    }
    Ok(())
}

#[test]
fn deterministic_lane_holds_hard_contradiction() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let west = school_id(&store, "Madison West")?;
    let east = school_id(&store, "Madison East")?;
    athlete_row(&store, &west, "Jordan Smith", Gender::Boys, "1001", 11)?;
    athlete_row(&store, &east, "Jordan Smith", Gender::Girls, "1001", 11)?;

    let report = reconcile_athletes(&store, "2026-09-22", false)?;
    if report.decided != 0 {
        return Err(format!("decided: {}", report.decided).into());
    }
    if report.pending != 1 {
        return Err(format!("pending: {}", report.pending).into());
    }
    if report.filed != 1 {
        return Err(format!("filed: {}", report.filed).into());
    }
    let cases: Vec<ReviewCase> = store.scan(Table::ReviewCases)?;
    if cases.len() != 1 {
        return Err(format!("review cases: {}", cases.len()).into());
    }
    if cases[0].state != ReviewState::Pending {
        return Err(format!("case state: {:?}", cases[0].state).into());
    }
    let verdicts: Vec<ReviewVerdictRecord> = store.scan(Table::IdentityVerdicts)?;
    if !verdicts.is_empty() {
        return Err(format!("hard contradictions file no verdicts: {}", verdicts.len()).into());
    }
    Ok(())
}

#[test]
fn alias_rows_file_without_deciding_and_dry_run_writes_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = school_id(&store, "Madison West")?;
    let mut athlete = CanonicalAthlete::new(
        &school,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1001"),
    )?;
    athlete.add_identity(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "1002",
    ));
    store.append(Table::Athletes, &athlete)?;

    let report = reconcile_athletes(&store, "2026-09-22", false)?;
    if report.aliases != 1 {
        return Err(format!("aliases: {}", report.aliases).into());
    }
    if report.filed != 1 {
        return Err(format!("filed: {}", report.filed).into());
    }
    if report.decided != 0 {
        return Err(format!("decided: {}", report.decided).into());
    }
    if report.pending != 1 {
        return Err(format!("pending: {}", report.pending).into());
    }
    let cases: Vec<ReviewCase> = store.scan(Table::ReviewCases)?;
    if cases.len() != 1 {
        return Err(format!("review cases: {}", cases.len()).into());
    }
    if cases[0].state != ReviewState::Pending {
        return Err(format!("case state: {:?}", cases[0].state).into());
    }

    let dry = tempfile::tempdir()?;
    let dry_store = Store::open(dry.path())?;
    let dry_school = school_id(&dry_store, "Madison West")?;
    let mut dry_athlete = CanonicalAthlete::new(
        &dry_school,
        "Jordan Smith",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1001"),
    )?;
    dry_athlete.add_identity(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "1002",
    ));
    dry_store.append(Table::Athletes, &dry_athlete)?;
    reconcile_athletes(&dry_store, "2026-09-22", true)?;
    let dry_cases: Vec<ReviewCase> = dry_store.scan(Table::ReviewCases)?;
    if !dry_cases.is_empty() {
        return Err(format!("a dry run files nothing: {}", dry_cases.len()).into());
    }
    Ok(())
}
