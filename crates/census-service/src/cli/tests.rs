//! What the clap surface must keep: the state flags resolve the way a caller reads them, and the
//! restriction a subcommand runs under is never inferred from what the flags happen to hold.
use super::{resolve_restriction, resolve_states};
use census_domain::UsJurisdiction;

/// `--all-states` is the census *run* scope (ADR-009), not every jurisdiction the domain models:
/// Alaska and Hawaii are valid values a run never acquires, so the flag must not admit them.
#[test]
fn all_states_selects_the_census_run_scope() {
    let states = resolve_states(true, &[]).expect("--all-states resolves");
    assert_eq!(states, UsJurisdiction::CENSUS_SCOPE);
    assert!(!states.contains(&UsJurisdiction::Alaska));
}

#[test]
fn no_flag_defaults_to_wisconsin() {
    let states = resolve_states(false, &[]).expect("the default resolves");
    assert_eq!(states, vec![UsJurisdiction::Wisconsin]);
}

#[test]
fn an_explicit_list_is_kept_in_caller_order() {
    let asked = vec![UsJurisdiction::Ohio, UsJurisdiction::Iowa];
    let states = resolve_states(false, &asked).expect("the list resolves");
    assert_eq!(states, asked);
}

#[test]
fn combining_the_two_flags_is_refused() {
    let error = resolve_states(true, &[UsJurisdiction::Ohio]).expect_err("both flags refused");
    assert!(error.to_string().contains("--all-states"));
}

/// The restriction form stays empty without a flag, so an adapter keeps its own coverage.
#[test]
fn a_restriction_with_no_flag_is_empty_not_wisconsin() {
    let states = resolve_restriction(false, &[]).expect("no restriction");
    assert!(states.is_empty());
    let all = resolve_restriction(true, &[]).expect("--all-states resolves");
    assert_eq!(all, UsJurisdiction::CENSUS_SCOPE);
    let explicit = resolve_restriction(false, &[UsJurisdiction::Ohio]).expect("explicit");
    assert_eq!(explicit, vec![UsJurisdiction::Ohio]);
    assert!(resolve_restriction(true, &[UsJurisdiction::Ohio]).is_err());
}
// ─── Verify subcommand acceptance tests ──────────────────────────────────────

use std::path::Path;

use census_domain::model::{
    CanonicalAthlete, CanonicalPerformance, EventKind, Gender, GradYear, Id, Mark, TeamId,
};
use rust_xlsxwriter::Workbook as Xlsx;

use super::verify::{run_verify, VerifyArgs};
use census_store::{Store, Table};

fn school(
    name: &str,
    state: census_domain::UsJurisdiction,
) -> (
    census_domain::model::CanonicalSchool,
    census_domain::model::SchoolId,
) {
    census_domain::model::CanonicalSchool::new(state, name, name.to_string().to_lowercase())
}

fn team_id(school: &str, sport: &str, gender: &str) -> TeamId {
    Id::mint("team", &[school, sport, gender])
}

fn write_test_workbook(
    dir: &Path,
    athletes: &[(String, String, String, String)],
    performances: &[(String, String, String)],
) -> std::path::PathBuf {
    let path = dir.join("verify-test.xlsx");
    let mut book = Xlsx::new();

    // Athletes sheet
    let athletes_sheet = book.add_worksheet();
    let _ = athletes_sheet.set_name("Athletes");
    let headers = ["Athlete ID", "Name", "School", "Graduation Year"];
    for (col, header) in headers.iter().enumerate() {
        let _ = athletes_sheet.write_string(0, u16::try_from(col).unwrap_or(0), *header);
    }
    for (row_idx, (aid, name, school, grad_year)) in athletes.iter().enumerate() {
        let row = u32::try_from(row_idx).unwrap_or(0).saturating_add(1);
        let _ = athletes_sheet.write_string(row, 0, aid);
        let _ = athletes_sheet.write_string(row, 1, name);
        let _ = athletes_sheet.write_string(row, 2, school);
        let _ = athletes_sheet.write_string(row, 3, grad_year);
    }

    // Performances sheet
    let perf_sheet = book.add_worksheet();
    let _ = perf_sheet.set_name("Performances_001");
    let perf_headers = ["Athlete ID", "Event", "Mark"];
    for (col, header) in perf_headers.iter().enumerate() {
        let _ = perf_sheet.write_string(0, u16::try_from(col).unwrap_or(0), *header);
    }
    for (row_idx, (aid, event, mark)) in performances.iter().enumerate() {
        let row = u32::try_from(row_idx).unwrap_or(0).saturating_add(1);
        let _ = perf_sheet.write_string(row, 0, aid);
        let _ = perf_sheet.write_string(row, 1, event);
        let _ = perf_sheet.write_string(row, 2, mark);
    }

    // Coverage sheet
    let coverage = book.add_worksheet();
    let _ = coverage.set_name("Coverage");
    let _ = coverage.write_string(0, 0, "state");
    let _ = coverage.write_string(1, 0, "WI");

    // Run Metrics sheet
    let metrics = book.add_worksheet();
    let _ = metrics.set_name("Run Metrics");
    let _ = metrics.write_string(0, 0, "metric");
    let _ = metrics.write_string(0, 1, "value");
    let _ = metrics.write_string(1, 0, "Class of 2027");
    let _ = metrics.write_number(1, 1, 0.0);

    let _ = book.save(&path);
    path
}

fn make_perf(
    athlete_id: &census_domain::model::AthleteId,
    team_id: &TeamId,
    event_kind: EventKind,
    time_seconds: u32,
) -> CanonicalPerformance {
    let event_id = Id::mint("evt", &["track", &format!("{event_kind:?}")]);
    let meet_id = Id::mint("meet", &["test-meet"]);
    let perf_id = CanonicalPerformance::mint(
        athlete_id,
        &meet_id,
        &event_kind,
        "2027-04-15",
        "test-source",
    );
    CanonicalPerformance {
        id: perf_id,
        athlete: athlete_id.clone(),
        team: team_id.clone(),
        event: event_id,
        meet: meet_id,
        date: "2027-04-15".to_string(),
        mark: Mark::TimeSeconds(time_seconds as f64),
        wind_mps: None,
        place: None,
        heat: None,
        round: None,
        timing: None,
        observed_grade: None,
        evidence: Vec::new(),
        source_key: "test".to_string(),
        source_athlete: None,
        retained_conflicts: Vec::new(),
    }
}

/// Agreement: store and workbook share the same rows → verify succeeds.
#[test]
fn acceptance_agreement() {
    let dir = tempfile::tempdir().unwrap();
    let (school_rec, school_id) =
        school("Jefferson High", census_domain::UsJurisdiction::Wisconsin);
    let team = team_id("Jefferson High", "track", "girls");

    let a1 = CanonicalAthlete::new(
        &school_id,
        "Alice Runner",
        GradYear::new(2027).unwrap(),
        Gender::Girls,
    );
    let a2 = CanonicalAthlete::new(
        &school_id,
        "Bob Sprinter",
        GradYear::new(2027).unwrap(),
        Gender::Boys,
    );

    let p1 = make_perf(&a1.id, &team, EventKind::Track200m, 26);
    let p2 = make_perf(&a2.id, &team, EventKind::Track100m, 11);

    let store = Store::open(dir.path()).expect("open temp store");
    store
        .append(Table::Schools, &school_rec)
        .expect("append school");
    store.append(Table::Athletes, &a1).expect("append athlete");
    store.append(Table::Athletes, &a2).expect("append athlete");
    store.append(Table::Performances, &p1).expect("append perf");
    store.append(Table::Performances, &p2).expect("append perf");

    let _ = write_test_workbook(
        dir.path(),
        &[
            (
                a1.id.as_str().to_string(),
                "Alice Runner".to_string(),
                school_id.as_str().to_string(),
                "2027".to_string(),
            ),
            (
                a2.id.as_str().to_string(),
                "Bob Sprinter".to_string(),
                school_id.as_str().to_string(),
                "2027".to_string(),
            ),
        ],
        &[
            (
                a1.id.as_str().to_string(),
                p1.event.as_str().to_string(),
                "26".to_string(),
            ),
            (
                a2.id.as_str().to_string(),
                p2.event.as_str().to_string(),
                "11".to_string(),
            ),
        ],
    );

    let args = VerifyArgs {
        workbook: Some(dir.path().join("verify-test.xlsx")),
        sample_every: 1,
        grad_year: 2027,
    };
    let result = run_verify(&store, &args);
    assert!(result.is_ok(), "verify should succeed: {:?}", result.err());
}

/// Disagreement: workbook has a wrong athlete name → verify fails.
#[test]
fn acceptance_disagreement() {
    let dir = tempfile::tempdir().unwrap();
    let (school_rec, school_id) =
        school("Jefferson High", census_domain::UsJurisdiction::Wisconsin);
    let a1 = CanonicalAthlete::new(
        &school_id,
        "Alice Runner",
        GradYear::new(2027).unwrap(),
        Gender::Girls,
    );

    let store = Store::open(dir.path()).expect("open temp store");
    store
        .append(Table::Schools, &school_rec)
        .expect("append school");
    store.append(Table::Athletes, &a1).expect("append athlete");

    // Write workbook with WRONG name.
    let _ = write_test_workbook(
        dir.path(),
        &[(
            a1.id.as_str().to_string(),
            "Alice Wrong".to_string(),
            school_id.as_str().to_string(),
            "2027".to_string(),
        )],
        &[],
    );

    let args = VerifyArgs {
        workbook: Some(dir.path().join("verify-test.xlsx")),
        sample_every: 1,
        grad_year: 2027,
    };
    let result = run_verify(&store, &args);
    assert!(result.is_err(), "verify should fail: got Ok");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Alice Wrong") || err.contains("not in store") || err.contains("not in store"),
        "error should mention the athlete row: {err}"
    );
}

/// Missing column: workbook Athletes sheet lacks Graduation Year → refused.
#[test]
fn acceptance_missing_column() {
    let dir = tempfile::tempdir().unwrap();

    let path = dir.path().join("verify-test.xlsx");
    let mut book = Xlsx::new();

    let athletes_sheet = book.add_worksheet();
    let _ = athletes_sheet.set_name("Athletes");
    let headers = ["Athlete ID", "Name", "School"]; // no Graduation Year
    for (col, header) in headers.iter().enumerate() {
        let _ = athletes_sheet.write_string(0, u16::try_from(col).unwrap_or(0), *header);
    }
    let _ = athletes_sheet.write_string(1, 0, "ath_test");
    let _ = athletes_sheet.write_string(1, 1, "Test Athlete");
    let _ = athletes_sheet.write_string(1, 2, "Jefferson High");

    // Empty Performances sheet with correct headers.
    let perf_sheet = book.add_worksheet();
    let _ = perf_sheet.set_name("Performances_001");
    let perf_headers = ["Athlete ID", "Event", "Mark"];
    for (col, header) in perf_headers.iter().enumerate() {
        let _ = perf_sheet.write_string(0, u16::try_from(col).unwrap_or(0), *header);
    }

    // Coverage and Run Metrics sheets.
    let coverage = book.add_worksheet();
    let _ = coverage.set_name("Coverage");
    let _ = coverage.write_string(0, 0, "state");
    let _ = coverage.write_string(1, 0, "WI");

    let metrics = book.add_worksheet();
    let _ = metrics.set_name("Run Metrics");
    let _ = metrics.write_string(0, 0, "metric");
    let _ = metrics.write_string(0, 1, "value");
    let _ = metrics.write_string(1, 0, "Class of 2027");
    let _ = metrics.write_number(1, 1, 0.0);

    let _ = book.save(&path);

    let store = Store::open(dir.path()).unwrap();
    let args = VerifyArgs {
        workbook: Some(path),
        sample_every: 1,
        grad_year: 2027,
    };
    let result = run_verify(&store, &args);
    assert!(result.is_err(), "verify should fail: got Ok");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Graduation Year"),
        "error should mention Graduation Year: {err}"
    );
}
