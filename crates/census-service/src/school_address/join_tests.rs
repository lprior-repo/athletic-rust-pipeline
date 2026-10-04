use std::collections::BTreeMap;

use census_domain::model::CanonicalSchool;
use census_domain::school_directory::{
    CityName, DirectoryIndex, Grade, GradeSpan, IdentifiedKey, NcesSchoolId, NumberedGrade,
    PostalAddress, SchoolDirectoryEntry, SchoolName, SourceLabel, StreetLine, Website, ZipCode,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

use super::*;
use crate::school_address::{CorpusReport, LaneReport, Report};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CAPTURE_SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const GENERATION: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

fn span(low: u8, high: u8) -> Result<GradeSpan, Box<dyn std::error::Error>> {
    Ok(GradeSpan::new(
        Grade::Numbered(NumberedGrade::new(low)?),
        Grade::Numbered(NumberedGrade::new(high)?),
    )?)
}

fn entry(
    id: &str,
    name: &str,
    city: &str,
    state: UsJurisdiction,
    line1: &str,
) -> Result<SchoolDirectoryEntry, Box<dyn std::error::Error>> {
    entry_with_website(id, name, city, state, line1, "")
}

fn entry_with_website(
    id: &str,
    name: &str,
    city: &str,
    state: UsJurisdiction,
    line1: &str,
    website: &str,
) -> Result<SchoolDirectoryEntry, Box<dyn std::error::Error>> {
    let address = PostalAddress::of(
        Some(StreetLine::parse(line1)?),
        None,
        Some(CityName::parse(city)?),
        Some(state),
        Some(ZipCode::parse("43000")?),
    )
    .ok_or("address is empty")?;
    Ok(SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse(id)?),
        SourceLabel::Ccd,
        Some(SchoolName::parse(name)?),
    )
    .with_address(Some(address))
    .with_website(Website::parse(website)?)
    .with_grades(Some(span(9, 12)?)))
}

fn store_with_school() -> Result<(tempfile::TempDir, Store), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Ohio,
        "Springfield High School",
        "springfield high school",
    );
    school.city = Some("Springfield".to_string());
    store.append(Table::Schools, &school)?;
    Ok((dir, store))
}

fn lanes(url: Option<&str>, observed_on: Option<&str>) -> BTreeMap<String, LaneEvidence> {
    BTreeMap::from([(
        "nces-ccd".to_string(),
        LaneEvidence {
            url: url.map(str::to_string),
            observed_on: observed_on.map(str::to_string),
            path: "research/sources/nces-ccd/raw/ccd.csv".to_string(),
            capture_sha256: CAPTURE_SHA.to_string(),
            generation: GENERATION.to_string(),
        },
    )])
}

fn merged_school(store: &Store) -> Result<CanonicalSchool, Box<dyn std::error::Error>> {
    let mut found: Option<CanonicalSchool> = None;
    store
        .snapshot()
        .for_each_merged(Table::Schools, |school: CanonicalSchool| {
            found = Some(school);
            Ok(())
        })?;
    found.ok_or_else(|| "no merged school".into())
}

#[test]
fn apply_links_a_matching_school_and_stamps_capture_evidence() -> TestResult {
    let (_dir, store) = store_with_school()?;
    let index = DirectoryIndex::build(&[entry_with_website(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "1 Main St",
        "https://springfield.example/high",
    )?]);
    let (counters, outcomes) = process(
        &store,
        &index,
        &lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01")),
        Mode::Apply,
    )?;
    check!(eq; counters.scanned, 1);
    check!(eq; counters.linked, 1);
    check!(eq; counters.websites, 1);
    check!(eq; counters.exact_name, 1);
    check!(eq; counters.evidence_missing, 0);
    check!(eq; outcomes.len(), 0);

    let merged = merged_school(&store)?;
    check!(eq; merged.postal_addresses.len(), 1);
    check!(eq; merged.school_website.as_deref(), Some("https://springfield.example/high"));
    check!(merged
        .evidence
        .iter()
        .any(|item| item.source.url.as_deref() == Some("https://nces.ed.gov/ccd.csv")));
    let claim = &merged.postal_addresses[0];
    check!(eq; claim.capture_sha256(), CAPTURE_SHA);
    check!(eq; claim.address().line1().map(|line| line.as_str()), Some("1 Main St"));
    check!(eq; claim.owner().id.as_str(), "390000000001");
    check!(eq; claim.source_label(), &SourceLabel::Ccd);
    check!(eq; claim.evidence().source.url.as_deref(), Some("https://nces.ed.gov/ccd.csv"));
    check!(eq; claim.evidence().observed_on.as_str(), "2026-08-01");
    check!(eq;
        claim.evidence().note.as_deref(),
        Some(format!("nces-ccd lane research/sources/nces-ccd/raw/ccd.csv sha256={CAPTURE_SHA} generation {GENERATION}").as_str())
    );
    check!(merged
        .source_identities
        .iter()
        .any(|identity| identity.id == "390000000001"));
    Ok(())
}

#[test]
fn a_second_apply_reports_already_linked_without_duplicating() -> TestResult {
    let (_dir, store) = store_with_school()?;
    let index = DirectoryIndex::build(&[entry(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "1 Main St",
    )?]);
    let lanes = lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01"));
    let (first, _) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; first.linked, 1);
    let (second, outcomes) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; second.linked, 0);
    check!(eq; second.already_linked, 1);
    check!(eq; outcomes.len(), 0);
    let merged = merged_school(&store)?;
    check!(eq; merged.postal_addresses.len(), 1);
    Ok(())
}

#[test]
fn an_existing_website_is_not_overwritten() -> TestResult {
    let (_dir, store) = store_with_school()?;
    let mut school = merged_school(&store)?;
    school.school_website = Some("https://mpa.example/school".to_string());
    store.append(Table::Schools, &school)?;
    let index = DirectoryIndex::build(&[entry_with_website(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "1 Main St",
        "https://springfield.example/high",
    )?]);
    let (counters, _) = process(
        &store,
        &index,
        &lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01")),
        Mode::Apply,
    )?;
    check!(eq; counters.linked, 1);
    check!(eq; counters.websites, 0);
    let merged = merged_school(&store)?;
    check!(eq; merged.school_website.as_deref(), Some("https://mpa.example/school"));
    Ok(())
}

#[test]
fn a_replay_backfills_the_website_for_an_already_linked_school() -> TestResult {
    let (_dir, store) = store_with_school()?;
    let lanes = lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01"));
    let first_index = DirectoryIndex::build(&[entry(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "1 Main St",
    )?]);
    let (first, _) = process(&store, &first_index, &lanes, Mode::Apply)?;
    check!(eq; first.linked, 1);
    check!(eq; first.websites, 0);
    let second_index = DirectoryIndex::build(&[entry_with_website(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "1 Main St",
        "https://springfield.example/high",
    )?]);
    let (second, outcomes) = process(&store, &second_index, &lanes, Mode::Apply)?;
    check!(eq; second.linked, 0);
    check!(eq; second.already_linked, 1);
    check!(eq; second.websites, 1);
    check!(eq; outcomes.len(), 0);
    let merged = merged_school(&store)?;
    check!(eq; merged.postal_addresses.len(), 1);
    check!(eq; merged.school_website.as_deref(), Some("https://springfield.example/high"));
    check!(eq;
        merged
            .evidence
            .iter()
            .filter(|item| item.source.url.as_deref() == Some("https://nces.ed.gov/ccd.csv"))
            .count(),
        1
    );
    Ok(())
}

#[test]
fn missing_capture_evidence_refuses_the_link() -> TestResult {
    let (_dir, store) = store_with_school()?;
    let index = DirectoryIndex::build(&[entry(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "1 Main St",
    )?]);
    for (url, observed_on) in [
        (None, None),
        (Some("https://nces.ed.gov/ccd.csv"), None),
        (None, Some("2026-08-01")),
    ] {
        let (counters, outcomes) = process(&store, &index, &lanes(url, observed_on), Mode::Apply)?;
        check!(eq; counters.linked, 0);
        check!(eq; counters.evidence_missing, 1);
        check!(eq; outcomes.len(), 1);
        check!(eq; outcomes[0].outcome.as_str(), "evidence_missing");
    }
    let merged = merged_school(&store)?;
    check!(eq; merged.postal_addresses.len(), 0);
    Ok(())
}

#[test]
fn dry_run_links_nothing_into_the_store() -> TestResult {
    let (_dir, store) = store_with_school()?;
    let index = DirectoryIndex::build(&[entry(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "1 Main St",
    )?]);
    let (counters, _) = process(
        &store,
        &index,
        &lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01")),
        Mode::DryRun,
    )?;
    check!(eq; counters.linked, 1);
    let merged = merged_school(&store)?;
    check!(eq; merged.postal_addresses.len(), 0);
    Ok(())
}

#[test]
fn overrides_reject_foreign_sources_and_bad_values() -> TestResult {
    let foreign = Overrides {
        urls: BTreeMap::from([("milesplit".to_string(), "https://x.test/a".to_string())]),
        dates: BTreeMap::new(),
    };
    check!(matches!(
        foreign.validated(),
        Err(JoinError::EvidenceSource { .. })
    ));
    let scheme = Overrides {
        urls: BTreeMap::from([("nces-ccd".to_string(), "ftp://x.test/a".to_string())]),
        dates: BTreeMap::new(),
    };
    check!(matches!(
        scheme.validated(),
        Err(JoinError::EvidenceValue { .. })
    ));
    let date = Overrides {
        urls: BTreeMap::new(),
        dates: BTreeMap::from([("nces-ccd".to_string(), "2026-13-40".to_string())]),
    };
    check!(matches!(
        date.validated(),
        Err(JoinError::EvidenceValue { .. })
    ));
    let accepted = Overrides {
        urls: BTreeMap::from([(
            "nces-ccd".to_string(),
            "https://nces.ed.gov/ccd.zip".to_string(),
        )]),
        dates: BTreeMap::from([("nces-ccd".to_string(), "2026-08-01".to_string())]),
    };
    check!(accepted.validated().is_ok());

    let pairs = parse_source_pairs(&["nces-ccd=https://nces.ed.gov/ccd.zip".to_string()])?;
    check!(eq; pairs.get("nces-ccd").map(String::as_str), Some("https://nces.ed.gov/ccd.zip"));
    check!(parse_source_pairs(&["nces-ccd".to_string()]).is_err());
    check!(parse_source_pairs(&["=https://nces.ed.gov/ccd.zip".to_string()]).is_err());
    Ok(())
}

#[test]
fn lane_evidence_reads_the_generation_lanes_and_overrides() -> TestResult {
    let report = Report {
        manifest_digest: GENERATION.to_string(),
        now: Some("2026-10".to_string()),
        lanes: vec![
            LaneReport {
                source: "nces-ccd".to_string(),
                path: "research/sources/nces-ccd/raw/ccd.csv".to_string(),
                sha256: CAPTURE_SHA.to_string(),
                entries: 3,
                skipped: 0,
                notes: 0,
                skipped_rows: Vec::new(),
                note_rows: Vec::new(),
            },
            LaneReport {
                source: "state-ed".to_string(),
                path: "research/sources/state-ed/index.html".to_string(),
                sha256: CAPTURE_SHA.to_string(),
                entries: 1,
                skipped: 0,
                notes: 0,
                skipped_rows: Vec::new(),
                note_rows: Vec::new(),
            },
        ],
        corpus: CorpusReport {
            rows: 4,
            entries: 2,
            skipped: 0,
            notes: 0,
            merges: 0,
        },
        changes: None,
        schedule: Vec::new(),
        outputs: Vec::new(),
        phases: None,
    };
    let overrides = Overrides {
        urls: BTreeMap::from([(
            "nces-ccd".to_string(),
            "https://nces.ed.gov/ccd.zip".to_string(),
        )]),
        dates: BTreeMap::from([("nces-ccd".to_string(), "2026-08-01".to_string())]),
    };
    let lanes = build_lane_evidence(&report, &overrides);
    check!(eq; lanes.len(), 1);
    let lane = lanes.get("nces-ccd").ok_or("missing nces-ccd lane")?;
    check!(eq; lane.url.as_deref(), Some("https://nces.ed.gov/ccd.zip"));
    check!(eq; lane.observed_on.as_deref(), Some("2026-08-01"));
    check!(eq; lane.capture_sha256.as_str(), CAPTURE_SHA);
    check!(eq; lane.generation.as_str(), GENERATION);

    let bare = build_lane_evidence(&report, &Overrides::default());
    let lane = bare.get("nces-ccd").ok_or("missing nces-ccd lane")?;
    check!(eq; lane.url, None);
    check!(eq; lane.observed_on, None);
    Ok(())
}

#[test]
fn a_missing_state_is_reported_instead_of_linked() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let (school, _) =
        CanonicalSchool::new(UsJurisdiction::Ohio, "Nowhere School", "nowhere school");
    let mut school = school;
    school.state = None;
    store.append(Table::Schools, &school)?;
    let index = DirectoryIndex::build(&[entry(
        "390000000001",
        "Nowhere School",
        "Nowhere",
        UsJurisdiction::Ohio,
        "1 Main St",
    )?]);
    let (counters, outcomes) = process(
        &store,
        &index,
        &lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01")),
        Mode::Apply,
    )?;
    check!(eq; counters.missing_state, 1);
    check!(eq; counters.linked, 0);
    check!(eq; outcomes.len(), 1);
    check!(eq; outcomes[0].reason.as_deref(), Some("missing_state"));
    Ok(())
}

#[test]
fn ambiguous_names_stay_in_review_with_candidate_labels() -> TestResult {
    let (_dir, store) = store_with_school()?;
    let index = DirectoryIndex::build(&[
        entry(
            "390000000002",
            "Springfield High School",
            "Springfield",
            UsJurisdiction::Ohio,
            "1 Main St",
        )?,
        entry(
            "390000000003",
            "Springfield High School",
            "Springfield",
            UsJurisdiction::Ohio,
            "2 Main St",
        )?,
    ]);
    let (counters, outcomes) = process(
        &store,
        &index,
        &lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01")),
        Mode::Apply,
    )?;
    check!(eq; counters.review, 1);
    check!(eq; counters.ambiguous, 1);
    check!(eq; outcomes.len(), 1);
    check!(eq; outcomes[0].outcome.as_str(), "review");
    check!(eq; outcomes[0].candidates.len(), 2);
    Ok(())
}

#[test]
fn another_states_school_never_links() -> TestResult {
    let (_dir, store) = store_with_school()?;
    let index = DirectoryIndex::build(&[entry(
        "390000000004",
        "Springfield High School",
        "Rochester Hills",
        UsJurisdiction::Michigan,
        "3 Main St",
    )?]);
    let (counters, _) = process(
        &store,
        &index,
        &lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01")),
        Mode::DryRun,
    )?;
    check!(eq; counters.scanned, 1);
    check!(eq; counters.no_match, 1);
    check!(eq; counters.linked, 0);
    Ok(())
}
