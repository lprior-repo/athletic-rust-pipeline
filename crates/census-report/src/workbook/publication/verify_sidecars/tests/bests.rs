use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CentiSeconds, CompetitionLevel, EventKind, Gender, GradYear, Id, Mark, SourceIdentity,
    SourceNamespace, Sport, TimingMethod,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

use crate::bests::{build_from_dataset, write};
use crate::export::ExportDataset;
use crate::workbook::Options;

use super::super::input::Inputs;
use super::rewrite_cell;

fn bests_options(options: &Options) -> crate::bests::Options {
    crate::bests::Options {
        scope: options.scope,
        grad_year: options.grad_year,
        limit: options.limit,
    }
}

#[test]
fn valid_best_results_sidecars_pass_and_forgeries_fail() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let store = Store::open(directory.path().join("store")).expect("own store");
    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Ada High", "ada high");
    store
        .append(Table::Schools, &school)
        .expect("school appends");
    let athlete = CanonicalAthlete::new(
        &school_id,
        "Ada Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "verify-1001"),
    );
    store
        .append(Table::Athletes, &athlete)
        .expect("athlete appends");

    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Ada Invite",
        "2026-05-02",
        CompetitionLevel::Unknown,
    );
    meet.sports = vec![Sport::OutdoorTrack];
    store.append(Table::Meets, &meet).expect("meet appends");
    let kind = EventKind::Track100m;
    let event = CanonicalEvent::new(&meet.id, kind.clone(), Gender::Boys, None, None);
    store.append(Table::Events, &event).expect("event appends");

    let mut slower = CanonicalPerformance {
        id: Id::mint("perf", &["verify-slower"]),
        athlete: athlete.id.clone(),
        team: Id::mint("team", &["verify-team"]),
        event: event.id.clone(),
        meet: meet.id.clone(),
        date: "2026-05-02".to_string(),
        mark: Mark::TimeSeconds(CentiSeconds::new(1100)),
        wind_mps: Some(1.0),
        place: None,
        heat: None,
        round: None,
        timing: Some(TimingMethod::Fat),
        observed_grade: None,
        evidence: vec![],
        source_key: "verify:slower".to_string(),
        source_athlete: athlete.source.clone(),
        retained_conflicts: vec![],
    };
    store
        .append(Table::Performances, &slower)
        .expect("slower appends");
    let mut faster = slower.clone();
    faster.id = Id::mint("perf", &["verify-faster"]);
    faster.source_key = "verify:faster".to_string();
    faster.mark = Mark::TimeSeconds(CentiSeconds::new(1080));
    store
        .append(Table::Performances, &faster)
        .expect("faster appends");

    let dataset = ExportDataset::load(&store).expect("frozen export loads");
    let options = Options::default();
    let rows = build_from_dataset(&dataset, &bests_options(&options));
    assert_eq!(rows.len(), 1);
    let inputs = Inputs::new(&dataset, &options).expect("inputs");

    let output = directory.path().join("bests");
    write(&output, &rows, "co2027").expect("best-results publish");
    super::super::bests::verify(&output, &inputs).expect("valid best-results verify");

    rewrite_cell(&output.join("best-results-co2027.csv"), "best_value", "999");
    let error =
        super::super::bests::verify(&output, &inputs).expect_err("forged best value refused");
    assert!(
        error.to_string().contains("best-results-co2027.csv"),
        "{error}"
    );

    let jsonl_output = directory.path().join("bests-jsonl");
    write(&jsonl_output, &rows, "co2027").expect("best-results publish");
    let jsonl = jsonl_output.join("best-results-co2027.jsonl");
    let line = std::fs::read_to_string(&jsonl).expect("best-results reads");
    std::fs::write(&jsonl, format!("{line}{line}")).expect("record duplicated");
    let error =
        super::super::bests::verify(&jsonl_output, &inputs).expect_err("duplicate record refused");
    assert!(error.to_string().contains("line 2"), "{error}");
}
