#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_domain::school_directory::SchoolDirectoryEntry;
use census_service::school_address::{self, Report, SchoolAddressArgs};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

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
        association_directory: Vec::new(),
        out: out.to_path_buf(),
        baseline: None,
        ledger: None,
        now: None,
        geocode: false,
        validate_postal: false,
    }
}

fn read_report(dir: &Path) -> TestResult<Report> {
    let generation = school_address::verify_current(dir)?;
    Ok(serde_json::from_slice(
        generation.artifact("pipeline_report.json")?,
    )?)
}

fn read(path: &Path) -> TestResult<Vec<u8>> {
    Ok(std::fs::read(path)?)
}

#[test]
fn school_address_reads_the_nces_fixtures_into_one_corpus() -> TestResult {
    let dir = TempDir::new()?;
    school_address::run(&args(dir.path()))?;

    let report = read_report(dir.path())?;
    check!(eq; report.corpus.rows, 1931);
    check!(eq; report.corpus.entries, 1931);
    check!(eq; report.corpus.skipped, 67);
    check!(eq; report.corpus.notes, 15);
    check!(eq; report.corpus.merges, 0);
    check!(eq; report.lanes.len(), 2);
    check!(eq; report.lanes[0].source, "nces-ccd");
    check!(eq; report.lanes[0].entries, 1557);
    check!(eq; report.lanes[0].skipped, 42);
    check!(eq; report.lanes[0].sha256,
    "e01af083baa57b60f37571f9607aa8ebe9b225712563ec9f10fd8809fab661c8");
    check!(eq; report.lanes[1].source, "nces-pss");
    check!(eq; report.lanes[1].entries, 374);
    check!(eq; report.lanes[1].skipped, 25);
    check!(report.lanes[1].note_rows.is_empty());

    let generation = school_address::verify_current(dir.path())?;
    let entries: Vec<SchoolDirectoryEntry> =
        serde_json::from_slice(generation.artifact("school_directory.json")?)?;
    check!(eq; entries.len(), 1931);
    check!(entries
        .iter()
        .any(|entry| entry.key().label() == "nces:010000500870"));
    check!(entries
        .iter()
        .any(|entry| entry.key().label() == "pss:A2380006"));

    let csv = std::str::from_utf8(generation.artifact("school_directory.csv")?)?;
    let lines: Vec<&str> = csv.lines().collect();
    check!(eq; lines.len(), 1932);
    check!(eq; lines[0],
    "key,name,kind,street1,street2,city,state,zip,phone,website,grades,enrollment,latitude,longitude,sources");
    check!(lines.contains(&"nces:010000500870,Albertville Middle School,Public,600 E Alabama Ave,,Albertville,AL,35950-2336,(256)878-2341,http://www.albertk12.org,7-8,,,,nces-ccd"));
    check!(csv.contains("nces-pss"));
    Ok(())
}

#[test]
fn school_address_writes_the_same_bytes_for_the_same_input() -> TestResult {
    let dir = TempDir::new()?;
    school_address::run(&args(dir.path()))?;
    let first_report = read(&dir.path().join("current/pipeline_report.json"))?;
    let first_entries = read(&dir.path().join("current/school_directory.json"))?;
    let first_csv = read(&dir.path().join("current/school_directory.csv"))?;

    school_address::run(&args(dir.path()))?;
    check!(eq; first_report,
    read(&dir.path().join("current/pipeline_report.json"))?);
    check!(eq; first_entries,
    read(&dir.path().join("current/school_directory.json"))?);
    check!(eq; first_csv,
    read(&dir.path().join("current/school_directory.csv"))?);
    Ok(())
}

#[test]
fn school_address_diffs_against_its_baseline_and_records_the_month() -> TestResult {
    let dir = TempDir::new()?;
    let baseline = dir.path().join("current/baseline.json");
    let ledger = dir.path().join("current/update_ledger.json");

    let mut first = args(dir.path());
    first.baseline = Some(baseline.clone());
    first.ledger = Some(ledger.clone());
    first.now = Some("2026-09".to_string());
    school_address::run(&first)?;

    let report = read_report(dir.path())?;
    check!(report.changes.is_none());
    check!(eq; report.schedule.len(), 2);
    check!(report
        .schedule
        .iter()
        .all(|decision| decision.decision == "due"));
    check!(!dir.path().join("current/changes.json").exists());
    let recorded: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&ledger)?)?;
    check!(eq; recorded["last"]["Ccd"], "2026-09");
    check!(eq; recorded["last"]["Pss"], "2026-09");

    let mut second = args(dir.path());
    second.baseline = Some(baseline);
    second.ledger = Some(ledger);
    second.now = Some("2026-10".to_string());
    school_address::run(&second)?;

    let report = read_report(dir.path())?;
    let changes = report
        .changes
        .ok_or("missing change set against baseline")?;
    check!(eq; changes.added, 0);
    check!(eq; changes.removed, 0);
    check!(eq; changes.modified, 0);
    check!(dir.path().join("current/changes.json").exists());
    check!(eq; report.now.as_deref(), Some("2026-10"));
    check!(eq; report.schedule[0].source, "nces-ccd");
    check!(eq; report.schedule[0].decision, "not-due");
    check!(eq; report.schedule[0].due, "2027-09");
    check!(eq; report.schedule[1].source, "nces-pss");
    check!(eq; report.schedule[1].decision, "not-due");
    check!(eq; report.schedule[1].due, "2028-01");
    Ok(())
}

struct EnvGuard {
    saved: Vec<(&'static str, Option<String>)>,
}

impl EnvGuard {
    fn clear(names: &[&'static str]) -> Self {
        let saved = names
            .iter()
            .map(|name| {
                let previous = std::env::var(name).ok();
                std::env::set_var(name, "");
                (*name, previous)
            })
            .collect();
        Self { saved }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (name, previous) in self.saved.drain(..) {
            match previous {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

#[test]
fn school_address_refuses_geocoding_without_the_google_credential() -> TestResult {
    let dir = TempDir::new()?;
    let guard = EnvGuard::clear(&["GOOGLE_MAPS_API_KEY", "GOOGLE_API_KEY"]);
    let mut geocode = args(dir.path());
    geocode.geocode = true;
    let error = match school_address::run(&geocode) {
        Err(error) => error,
        Ok(_) => return Err("geocoding without credential accepted".into()),
    };
    let message = format!("{error:#}");
    check!(message.contains("GOOGLE_MAPS_API_KEY"), "{message}");
    check!(!dir.path().join("pipeline_report.json").exists());
    drop(guard);
    Ok(())
}

#[test]
fn school_address_refuses_postal_validation_without_the_usps_credential() -> TestResult {
    let dir = TempDir::new()?;
    let guard = EnvGuard::clear(&["USPS_API_TOKEN"]);
    let mut postal = args(dir.path());
    postal.validate_postal = true;
    let error = match school_address::run(&postal) {
        Err(error) => error,
        Ok(_) => return Err("postal validation without credential accepted".into()),
    };
    let message = format!("{error:#}");
    check!(message.contains("USPS_API_TOKEN"), "{message}");
    check!(!dir.path().join("pipeline_report.json").exists());
    drop(guard);
    Ok(())
}

#[test]
fn school_address_refuses_a_diff_without_a_run_month() -> TestResult {
    let dir = TempDir::new()?;
    let mut monthless = args(dir.path());
    monthless.baseline = Some(dir.path().join("baseline.json"));
    let error = match school_address::run(&monthless) {
        Err(error) => error,
        Ok(_) => return Err("monthless diff accepted".into()),
    };
    check!(format!("{error:#}").contains("--now"));
    Ok(())
}

#[test]
fn school_address_refuses_an_artifact_of_the_wrong_shape() -> TestResult {
    let dir = TempDir::new()?;
    let mut swapped = args(dir.path());
    swapped.ccd = Some(PathBuf::from(PSS));
    let error = match school_address::run(&swapped) {
        Err(error) => error,
        Ok(_) => return Err("wrong artifact shape accepted".into()),
    };
    let message = format!("{error:#}");
    check!(message.contains("NCESSCH"), "{message}");
    check!(message.contains("pss2324_pu_head.csv"), "{message}");
    check!(!dir.path().join("school_directory.csv").exists());
    Ok(())
}

#[test]
fn school_address_refuses_a_baseline_that_names_an_export() -> TestResult {
    let dir = TempDir::new()?;
    let mut args = args(dir.path());
    args.baseline = Some(dir.path().join("school_directory.json"));
    args.now = Some("2026-09".to_string());
    let error = match school_address::run(&args) {
        Err(error) => error,
        Ok(_) => return Err("export destination accepted as baseline".into()),
    };
    check!(
        error.to_string().contains("two outputs name"),
        "the refusal names the collision: {error}"
    );
    check!(
        !dir.path().join("school_directory.json").exists(),
        "the destinations are checked before any artifact is published"
    );
    Ok(())
}
