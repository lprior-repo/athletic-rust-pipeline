use super::*;
use census_domain::model::SchoolYear;

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn team(id: &str) -> TeamRef {
    TeamRef {
        id: id.to_string(),
        slug: "example".to_string(),
        url: format!("https://wi.milesplit.com/teams/{id}-example"),
        name: "Example School".to_string(),
        city_state: "Madison, WI".to_string(),
    }
}
fn read(teams: Vec<TeamRef>) -> TeamIndexRead {
    TeamIndexRead {
        teams,
        disposition: Disposition::Complete,
        unfinished: Vec::new(),
        errors: 0,
    }
}
fn run() -> Result<CensusRun, Box<dyn std::error::Error>> {
    CensusRun::new(SchoolYear::DEFAULT, 1).ok_or_else(|| "invalid run".into())
}

#[test]
fn legacy_zero_index_does_not_erase_unmeasured_original_inventory() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let jurisdiction = UsJurisdiction::Wisconsin;
    store.journal_done(
        &teams_phase(jurisdiction),
        "WI",
        &serde_json::json!({ "teams": 0, "host": "wi.milesplit.com" }),
    )?;
    configure(&store, jurisdiction, read(vec![team("52649")]))?;
    configure(&store, jurisdiction, read(vec![team("52649")]))?;
    let evidence = inspect_rosters(&store, run()?, jurisdiction)?;
    check!(eq; evidence.disposition, Disposition::Partial);
    check!(evidence
        .objects
        .iter()
        .any(
            |object| object.endpoint.contains("legacy-inventory-unmeasured") && !object.terminal()
        ));
    check!(eq; evidence.owed, 1);
    Ok(())
}

#[test]
fn missing_original_input_is_named_owed_and_never_fabricated_from_fresh_directory() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let jurisdiction = UsJurisdiction::Wisconsin;
    save(
        &store,
        jurisdiction,
        &Receipt {
            roster_teams: vec!["52649".to_string()],
            disposition: Disposition::Partial,
            ..Receipt::default()
        },
    )?;
    let configured = configure(&store, jurisdiction, read(vec![team("52650")]))?;
    check!(eq; configured.into_iter().map(|team| team.id).collect::<Vec<_>>(), vec!["52650".to_string()]);
    let evidence = inspect_rosters(&store, run()?, jurisdiction)?;
    check!(eq; (evidence.disposition, evidence.owed), (Disposition::Partial, 2));
    check!(evidence
        .objects
        .iter()
        .any(|object| object.endpoint == "WI/roster-input/52649/unmeasured" && !object.terminal()));
    Ok(())
}

#[test]
fn index_partial_prefix_and_exact_late_locator_remain_owed() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let locator = "https://wi.milesplit.com/teams#row-20001-bytes-1024";
    configure(
        &store,
        UsJurisdiction::Wisconsin,
        TeamIndexRead {
            teams: vec![team("52649")],
            disposition: Disposition::Partial,
            unfinished: vec![locator.to_string()],
            errors: 1,
        },
    )?;
    let receipt = load(&store, UsJurisdiction::Wisconsin)?.ok_or("missing index")?;
    check!(eq; receipt.unfinished, vec![locator.to_string()]);
    let evidence = inspect_rosters(&store, run()?, UsJurisdiction::Wisconsin)?;
    check!(eq; evidence.disposition, Disposition::Partial);
    check!(evidence.objects.iter().any(|row| row.endpoint
        == "https://wi.milesplit.com/teams/52649-example/roster"
        && !row.terminal()));
    Ok(())
}

#[test]
fn completed_directory_report_does_not_wait_for_roster_collection() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let jurisdiction = UsJurisdiction::Wisconsin;
    configure(&store, jurisdiction, read(vec![team("52649")]))?;
    let report = team_index_report(&store, jurisdiction)?;
    check!(eq; report.disposition, Disposition::Complete);
    check!(eq; report.rows, 1);
    check!(eq; report.errors, 0);
    check!(report.unfinished.is_empty());
    let rosters = inspect_rosters(&store, run()?, jurisdiction)?;
    check!(eq; rosters.disposition, Disposition::Partial);
    check!(eq; rosters.owed, 1);
    Ok(())
}

#[test]
fn partial_directory_report_preserves_acquisition_failure_and_locator() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let jurisdiction = UsJurisdiction::Wisconsin;
    let locator = "https://wi.milesplit.com/teams#row-20001-bytes-1024";
    configure(
        &store,
        jurisdiction,
        TeamIndexRead {
            teams: vec![team("52649")],
            disposition: Disposition::Partial,
            unfinished: vec![locator.to_string()],
            errors: 1,
        },
    )?;
    let report = team_index_report(&store, jurisdiction)?;
    check!(eq; report.disposition, Disposition::Partial);
    check!(eq; report.rows, 1);
    check!(eq; report.errors, 1);
    check!(eq; report.unfinished, vec![locator.to_string()]);
    Ok(())
}

#[test]
fn missing_durable_team_input_refuses_completed_directory_report() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let jurisdiction = UsJurisdiction::Wisconsin;
    check!(eq; team_index_report(&store, jurisdiction)?.disposition, Disposition::Unknown);
    save(
        &store,
        jurisdiction,
        &Receipt {
            roster_teams: vec!["52649".to_string()],
            disposition: Disposition::Complete,
            ..Receipt::default()
        },
    )?;
    let report = team_index_report(&store, jurisdiction)?;
    check!(eq; report.disposition, Disposition::Partial);
    check!(eq; report.rows, 0);
    check!(eq; report.unfinished, vec!["WI/roster-input/52649/unmeasured".to_string()]);
    Ok(())
}
