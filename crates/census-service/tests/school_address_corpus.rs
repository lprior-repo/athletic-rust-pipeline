use census_domain::school_directory::SchoolDirectoryEntry;
use census_service::school_address::{self, Report, SchoolAddressArgs};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const CCD: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/tests/fixtures/nces/ccd_sch_029_2526_head.csv"
);
const PSS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/tests/fixtures/nces/pss2324_pu_head.csv"
);

fn args(out: &Path) -> SchoolAddressArgs {
    SchoolAddressArgs {
        ccd: Some(PathBuf::from(CCD)),
        pss: Some(PathBuf::from(PSS)),
        state_ed_index: Vec::new(),
        state_ed_profile: Vec::new(),
        state_ed_tabular: Vec::new(),
        associations: Vec::new(),
        out: out.to_path_buf(),
        baseline: None,
        ledger: None,
        now: None,
        geocode: false,
        validate_postal: false,
    }
}

fn read_report(dir: &Path) -> Report {
    let generation = school_address::verify_current(dir).expect("verified generation");
    serde_json::from_slice(generation.artifact("pipeline_report.json").expect("report"))
        .expect("a parsable report")
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

#[test]
fn school_address_reads_the_nces_fixtures_into_one_corpus() {
    let dir = TempDir::new().expect("a temporary output directory");
    school_address::run(&args(dir.path())).expect("the committed fixtures read");

    let report = read_report(dir.path());
    assert_eq!(report.corpus.rows, 1931);
    assert_eq!(report.corpus.entries, 1931);
    assert_eq!(report.corpus.skipped, 67);
    assert_eq!(report.corpus.notes, 0);
    assert_eq!(report.corpus.merges, 0);
    assert_eq!(report.lanes.len(), 2);
    assert_eq!(report.lanes[0].source, "nces-ccd");
    assert_eq!(report.lanes[0].entries, 1557);
    assert_eq!(report.lanes[0].skipped, 42);
    assert_eq!(
        report.lanes[0].sha256,
        "e01af083baa57b60f37571f9607aa8ebe9b225712563ec9f10fd8809fab661c8"
    );
    assert_eq!(report.lanes[1].source, "nces-pss");
    assert_eq!(report.lanes[1].entries, 374);
    assert_eq!(report.lanes[1].skipped, 25);
    assert!(report.lanes[1].note_rows.is_empty());

    let generation = school_address::verify_current(dir.path()).expect("verified generation");
    let entries: Vec<SchoolDirectoryEntry> = serde_json::from_slice(
        generation
            .artifact("school_directory.json")
            .expect("entries"),
    )
    .expect("the entries parse");
    assert_eq!(entries.len(), 1931);
    assert!(entries
        .iter()
        .any(|entry| entry.key().label() == "nces:010000500870"));
    assert!(entries
        .iter()
        .any(|entry| entry.key().label() == "pss:A2380006"));

    let csv = std::str::from_utf8(generation.artifact("school_directory.csv").expect("csv"))
        .expect("utf8 csv");
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 1932);
    assert_eq!(
        lines[0],
        "key,name,kind,street1,street2,city,state,zip,phone,website,grades,enrollment,latitude,longitude,sources"
    );
    assert!(lines
        .iter()
        .any(|line| *line == "nces:010000500870,Albertville Middle School,Public,600 E Alabama Ave,,Albertville,AL,35950-2336,(256)878-2341,,7-8,,,,nces-ccd"));
    assert!(csv.contains("nces-pss"));
}

#[test]
fn school_address_writes_the_same_bytes_for_the_same_input() {
    let dir = TempDir::new().expect("a temporary output directory");
    school_address::run(&args(dir.path())).expect("the first run");
    let first_report = read(&dir.path().join("current/pipeline_report.json"));
    let first_entries = read(&dir.path().join("current/school_directory.json"));
    let first_csv = read(&dir.path().join("current/school_directory.csv"));

    school_address::run(&args(dir.path())).expect("the second run");
    assert_eq!(
        first_report,
        read(&dir.path().join("current/pipeline_report.json"))
    );
    assert_eq!(
        first_entries,
        read(&dir.path().join("current/school_directory.json"))
    );
    assert_eq!(
        first_csv,
        read(&dir.path().join("current/school_directory.csv"))
    );
}

#[test]
fn school_address_diffs_against_its_baseline_and_records_the_month() {
    let dir = TempDir::new().expect("a temporary output directory");
    let baseline = dir.path().join("current/baseline.json");
    let ledger = dir.path().join("current/update_ledger.json");

    let mut first = args(dir.path());
    first.baseline = Some(baseline.clone());
    first.ledger = Some(ledger.clone());
    first.now = Some("2026-09".to_string());
    school_address::run(&first).expect("the first run");

    let report = read_report(dir.path());
    assert!(report.changes.is_none());
    assert_eq!(report.schedule.len(), 2);
    assert!(report
        .schedule
        .iter()
        .all(|decision| decision.decision == "due"));
    assert!(!dir.path().join("current/changes.json").exists());
    let recorded: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&ledger).expect("the ledger"))
            .expect("a parsable ledger");
    assert_eq!(recorded["last"]["Ccd"], "2026-09");
    assert_eq!(recorded["last"]["Pss"], "2026-09");

    let mut second = args(dir.path());
    second.baseline = Some(baseline);
    second.ledger = Some(ledger);
    second.now = Some("2026-10".to_string());
    school_address::run(&second).expect("the second run");

    let report = read_report(dir.path());
    let changes = report.changes.expect("a change set against the baseline");
    assert_eq!(changes.added, 0);
    assert_eq!(changes.removed, 0);
    assert_eq!(changes.modified, 0);
    assert!(dir.path().join("current/changes.json").exists());
    assert_eq!(report.now.as_deref(), Some("2026-10"));
    assert_eq!(report.schedule[0].source, "nces-ccd");
    assert_eq!(report.schedule[0].decision, "not-due");
    assert_eq!(report.schedule[0].due, "2027-09");
    assert_eq!(report.schedule[1].source, "nces-pss");
    assert_eq!(report.schedule[1].decision, "not-due");
    assert_eq!(report.schedule[1].due, "2028-01");
}

#[test]
fn school_address_refuses_the_unbuilt_geocode_phases() {
    let dir = TempDir::new().expect("a temporary output directory");
    let mut geocode = args(dir.path());
    geocode.geocode = true;
    let error = school_address::run(&geocode).expect_err("geocoding is not built");
    assert!(format!("{error:#}").contains("census-crawl::geocode"));
    assert!(!dir.path().join("pipeline_report.json").exists());

    let mut postal = args(dir.path());
    postal.validate_postal = true;
    let error = school_address::run(&postal).expect_err("postal validation is not built");
    assert!(format!("{error:#}").contains("census-crawl::geocode"));
}

#[test]
fn school_address_refuses_a_diff_without_a_run_month() {
    let dir = TempDir::new().expect("a temporary output directory");
    let mut monthless = args(dir.path());
    monthless.baseline = Some(dir.path().join("baseline.json"));
    let error = school_address::run(&monthless).expect_err("a diff needs a month");
    assert!(format!("{error:#}").contains("--now"));
}

#[test]
fn school_address_refuses_an_artifact_of_the_wrong_shape() {
    let dir = TempDir::new().expect("a temporary output directory");
    let mut swapped = args(dir.path());
    swapped.ccd = Some(PathBuf::from(PSS));
    let error = school_address::run(&swapped).expect_err("the pss window is not a ccd file");
    let message = format!("{error:#}");
    assert!(message.contains("NCESSCH"), "{message}");
    assert!(message.contains("pss2324_pu_head.csv"), "{message}");
    assert!(!dir.path().join("school_directory.csv").exists());
}

#[test]
fn school_address_refuses_a_baseline_that_names_an_export() {
    let dir = TempDir::new().expect("a temporary output directory");
    let mut args = args(dir.path());
    args.baseline = Some(dir.path().join("school_directory.json"));
    args.now = Some("2026-09".to_string());
    let error =
        school_address::run(&args).expect_err("an export destination cannot also be the baseline");
    assert!(
        error.to_string().contains("two outputs name"),
        "the refusal names the collision: {error}"
    );
    assert!(
        !dir.path().join("school_directory.json").exists(),
        "the destinations are checked before any artifact is published"
    );
}
