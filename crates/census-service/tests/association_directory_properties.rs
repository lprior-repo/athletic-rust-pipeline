use census_crawl::tssaa;
use census_domain::school_directory::{DirectoryKey, SchoolDirectoryEntry, SourceLabel};
use census_domain::UsJurisdiction;
use census_service::school_address::{self, Report, SchoolAddressArgs};
use std::collections::BTreeMap;
use std::path::Path;
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const DIRECTORY: &str =
    include_str!("../../census-crawl/tests/fixtures/tssaa/directory_id157.html");
const FIXTURE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/tests/fixtures/tssaa/directory_id157.html"
);

fn args(out: &Path, association_directory: Vec<String>) -> SchoolAddressArgs {
    SchoolAddressArgs {
        ccd: None,
        pss: None,
        state_ed_index: Vec::new(),
        state_ed_profile: Vec::new(),
        state_ed_tabular: Vec::new(),
        associations: Vec::new(),
        association_directory,
        out: out.to_path_buf(),
        baseline: None,
        ledger: None,
        now: None,
        geocode: false,
        validate_postal: false,
    }
}

#[test]
fn tssaa_directory_publishes_tennessee_records_with_city_only_addresses() -> TestResult {
    let outcome = tssaa::parse_school_list(DIRECTORY)?;
    if outcome.entries().len() != 456 {
        return Err(format!("entry count: left={}, right=456", outcome.entries().len()).into());
    }
    outcome
        .entries()
        .iter()
        .try_for_each(|entry| -> TestResult {
            match entry.key() {
                DirectoryKey::StateRecord { state, .. } if state == &UsJurisdiction::Tennessee => {}
                other => {
                    return Err(format!("expected Tennessee StateRecord key, got {other:?}").into())
                }
            }
            let source = SourceLabel::AthleticAssociation {
                state: UsJurisdiction::Tennessee,
            };
            if !entry.sources().contains(&source) {
                return Err(format!(
                    "missing Tennessee association source: {:?}",
                    entry.sources()
                )
                .into());
            }
            if entry
                .address()
                .is_some_and(|address| address.line1().is_some() || address.line2().is_some())
            {
                return Err(
                    format!("directory invented a street for {}", entry.key().label()).into(),
                );
            }
            Ok(())
        })?;
    if !outcome
        .entries()
        .iter()
        .any(|entry| entry.address().and_then(|address| address.city()).is_some())
    {
        return Err("directory lost every published city".into());
    }
    Ok(())
}

#[test]
fn association_directory_alone_publishes_the_reader_entries_and_slug_token() -> TestResult {
    let out = TempDir::new()?;
    let request = args(out.path(), vec![format!("tssaa={FIXTURE_PATH}")]);
    school_address::run(&request)?;

    let report: Report = serde_json::from_slice(&std::fs::read(
        out.path().join("current/pipeline_report.json"),
    )?)?;
    if report.lanes.len() != 1 {
        return Err(format!("lane count: left={}, right=1", report.lanes.len()).into());
    }
    let lane = report.lanes.first().ok_or("missing association lane")?;
    if lane.source != "association:tssaa" || lane.entries != 456 {
        return Err(format!(
            "association lane source/entries: left={:?}/{}, right=association:tssaa/456",
            lane.source, lane.entries
        )
        .into());
    }
    let entries: Vec<SchoolDirectoryEntry> = serde_json::from_slice(&std::fs::read(
        out.path().join("current/school_directory.json"),
    )?)?;
    if entries.len() != 456 {
        return Err(format!("published entry count: left={}, right=456", entries.len()).into());
    }
    let first = entries.first().ok_or("missing first published entry")?;
    if !matches!(first.key(), DirectoryKey::StateRecord { .. }) {
        return Err(format!("expected first StateRecord key, got {:?}", first.key()).into());
    }
    let outcome = tssaa::parse_school_list(DIRECTORY)?;
    let expected: BTreeMap<_, _> = outcome
        .entries()
        .iter()
        .map(|entry| (entry.key(), entry))
        .collect();
    let published: BTreeMap<_, _> = entries.iter().map(|entry| (entry.key(), entry)).collect();
    if published != expected {
        return Err("published entries differ from the admitted reader's entries".into());
    }
    Ok(())
}

#[test]
fn association_directory_rejects_invalid_pairs_and_names_admitted_slugs() -> TestResult {
    let pairs = [
        "tssaa".to_string(),
        "=path".to_string(),
        "TSSAA=path".to_string(),
        "nope=path".to_string(),
        format!("{}=path", "a".repeat(65)),
        "tssaa.=path".to_string(),
        "tssáa=path".to_string(),
    ];
    pairs.into_iter().try_for_each(|pair| -> TestResult {
        let out = TempDir::new()?;
        let error = match school_address::run(&args(out.path(), vec![pair.clone()])) {
            Err(error) => error,
            Ok(()) => return Err(format!("invalid association pair accepted: {pair:?}").into()),
        };
        let message = error.to_string();
        if !message.contains("--association-directory")
            || !message.contains("admitted slugs: tssaa")
        {
            return Err(format!("pair {pair:?} did not name the admitted slugs: {message}").into());
        }
        Ok(())
    })
}
