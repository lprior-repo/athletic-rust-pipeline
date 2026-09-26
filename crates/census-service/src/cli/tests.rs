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

use std::path::Path;

use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalPerformance, CentiSeconds, EventKind, Gender,
    GradYear, Id, Mark, TeamId,
};
use census_report::bests::mark_text;
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
    path
}

/// The event a test's performances share: the store holds this row, so the sheet prints its label.
fn make_event(kind: EventKind, gender: Gender) -> CanonicalEvent {
    let meet_id = Id::mint("meet", &["test-meet"]);
    CanonicalEvent::new(&meet_id, kind, gender, None, None)
}

/// A performance in `event`: the fixture references the event row the store holds, so the sheet's
/// Event cell and the store's key state the same fact.
fn make_perf(
    athlete_id: &census_domain::model::AthleteId,
    team_id: &TeamId,
    event: &CanonicalEvent,
    time_seconds: u32,
) -> CanonicalPerformance {
    let perf_id = CanonicalPerformance::mint(
        athlete_id,
        &event.meet,
        &event.kind,
        "2027-04-15",
        "test-source",
    );
    CanonicalPerformance {
        id: perf_id,
        athlete: athlete_id.clone(),
        team: team_id.clone(),
        event: event.id.clone(),
        meet: event.meet.clone(),
        date: "2027-04-15".to_string(),
        mark: Mark::TimeSeconds(
            CentiSeconds::try_from_seconds_f64(time_seconds as f64).expect("fixture is in range"),
        ),
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

    let e1 = make_event(EventKind::Track200m, Gender::Girls);
    let e2 = make_event(EventKind::Track100m, Gender::Boys);
    let p1 = make_perf(&a1.id, &team, &e1, 26);
    let p2 = make_perf(&a2.id, &team, &e2, 11);

    let store = Store::open(dir.path()).expect("open temp store");
    store
        .append(Table::Schools, &school_rec)
        .expect("append school");
    store.append(Table::Athletes, &a1).expect("append athlete");
    store.append(Table::Athletes, &a2).expect("append athlete");
    store.append(Table::Events, &e1).expect("append event");
    store.append(Table::Events, &e2).expect("append event");
    store.append(Table::Performances, &p1).expect("append perf");
    store.append(Table::Performances, &p2).expect("append perf");

    let _ = write_test_workbook(
        dir.path(),
        &[
            (
                a1.id.as_str().to_string(),
                "Alice Runner".to_string(),
                school_rec.name.clone(),
                "2027".to_string(),
            ),
            (
                a2.id.as_str().to_string(),
                "Bob Sprinter".to_string(),
                school_rec.name.clone(),
                "2027".to_string(),
            ),
        ],
        &[
            (
                a1.id.as_str().to_string(),
                e1.kind.stable_key().to_string(),
                mark_text(&p1.mark),
            ),
            (
                a2.id.as_str().to_string(),
                e2.kind.stable_key().to_string(),
                mark_text(&p2.mark),
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

/// The performances sheet's version of the school-id leak: the Event cell carries the event's raw
/// id where the store holds the row and therefore a label. The sheet prints labels, so `verify`
/// refuses it.
#[test]
fn acceptance_event_id_where_the_store_has_a_label() {
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
    let e1 = make_event(EventKind::Track200m, Gender::Girls);
    let p1 = make_perf(&a1.id, &team, &e1, 26);

    let store = Store::open(dir.path()).expect("open temp store");
    store
        .append(Table::Schools, &school_rec)
        .expect("append school");
    store.append(Table::Athletes, &a1).expect("append athlete");
    store.append(Table::Events, &e1).expect("append event");
    store.append(Table::Performances, &p1).expect("append perf");

    let _ = write_test_workbook(
        dir.path(),
        &[(
            a1.id.as_str().to_string(),
            "Alice Runner".to_string(),
            school_rec.name.clone(),
            "2027".to_string(),
        )],
        &[(
            a1.id.as_str().to_string(),
            p1.event.as_str().to_string(),
            mark_text(&p1.mark),
        )],
    );

    let args = VerifyArgs {
        workbook: Some(dir.path().join("verify-test.xlsx")),
        sample_every: 1,
        grad_year: 2027,
    };
    let result = run_verify(&store, &args);
    let err = result
        .expect_err("an id printed where the store holds a label is a disagreement")
        .to_string();
    assert!(
        err.contains(EventKind::Track200m.stable_key().as_ref()),
        "the refusal should name the store's event label: {err}"
    );
}

/// The documented fallback: the store holds no event row, so the sheet has no label to print and
/// leaves the Event cell empty. `verify` accepts exactly that, and nothing wider.
#[test]
fn acceptance_empty_event_cell_with_no_store_row() {
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
    let e1 = make_event(EventKind::Track200m, Gender::Girls);
    let p1 = make_perf(&a1.id, &team, &e1, 26);

    let store = Store::open(dir.path()).expect("open temp store");
    store
        .append(Table::Schools, &school_rec)
        .expect("append school");
    store.append(Table::Athletes, &a1).expect("append athlete");
    store.append(Table::Performances, &p1).expect("append perf");

    let _ = write_test_workbook(
        dir.path(),
        &[(
            a1.id.as_str().to_string(),
            "Alice Runner".to_string(),
            school_rec.name.clone(),
            "2027".to_string(),
        )],
        &[(
            a1.id.as_str().to_string(),
            String::new(),
            mark_text(&p1.mark),
        )],
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

/// The shape a run-scope leak published: the School cell carries the school's raw id where the
/// store holds the row and therefore a name. The sheet prints names, so `verify` refuses it — the
/// check that caught the 2026-09-23 export's out-of-scope rows.
#[test]
fn acceptance_school_id_where_the_store_has_a_name() {
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

    let _ = write_test_workbook(
        dir.path(),
        &[(
            a1.id.as_str().to_string(),
            "Alice Runner".to_string(),
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
    let err = result
        .expect_err("an id printed where the store holds a name is a disagreement")
        .to_string();
    assert!(
        err.contains("Jefferson High"),
        "the refusal should name the store's school: {err}"
    );
}

/// The documented fallback: the store holds no row for the athlete's school id, so the sheet has no
/// name to print and carries the id itself. `verify` accepts exactly that, and nothing wider.
#[test]
fn acceptance_school_id_with_no_store_row() {
    let dir = tempfile::tempdir().unwrap();
    let (school_rec, _) = school("Jefferson High", census_domain::UsJurisdiction::Wisconsin);
    let ghost_id: census_domain::model::SchoolId = Id::mint("school", &["Ghost High"]);
    let a1 = CanonicalAthlete::new(
        &ghost_id,
        "Ghost Runner",
        GradYear::new(2027).unwrap(),
        Gender::Girls,
    );

    let store = Store::open(dir.path()).expect("open temp store");
    store
        .append(Table::Schools, &school_rec)
        .expect("append school");
    store.append(Table::Athletes, &a1).expect("append athlete");

    let _ = write_test_workbook(
        dir.path(),
        &[(
            a1.id.as_str().to_string(),
            "Ghost Runner".to_string(),
            ghost_id.as_str().to_string(),
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
    assert!(result.is_ok(), "verify should succeed: {:?}", result.err());
}

/// Missing column: workbook Athletes sheet lacks Graduation Year → refused.
#[test]
fn acceptance_missing_column() {
    let dir = tempfile::tempdir().unwrap();

    let path = dir.path().join("verify-test.xlsx");
    let mut book = Xlsx::new();

    let athletes_sheet = book.add_worksheet();
    let _ = athletes_sheet.set_name("Athletes");
    let headers = ["Athlete ID", "Name", "School"];
    for (col, header) in headers.iter().enumerate() {
        let _ = athletes_sheet.write_string(0, u16::try_from(col).unwrap_or(0), *header);
    }
    let _ = athletes_sheet.write_string(1, 0, "ath_test");
    let _ = athletes_sheet.write_string(1, 1, "Test Athlete");
    let _ = athletes_sheet.write_string(1, 2, "Jefferson High");

    let perf_sheet = book.add_worksheet();
    let _ = perf_sheet.set_name("Performances_001");
    let perf_headers = ["Athlete ID", "Event", "Mark"];
    for (col, header) in perf_headers.iter().enumerate() {
        let _ = perf_sheet.write_string(0, u16::try_from(col).unwrap_or(0), *header);
    }

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
