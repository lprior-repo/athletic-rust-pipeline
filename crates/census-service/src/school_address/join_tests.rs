use std::collections::BTreeMap;

use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalSchool, Gender, GradYear, ReviewCase, ReviewState,
    SchoolId, SourceIdentity, SourceNamespace,
};
use census_domain::school_directory::{
    CityName, DirectoryIndex, Grade, GradeSpan, IdentifiedKey, NcesSchoolId, NumberedGrade,
    PostalAddress, SchoolDirectoryEntry, SchoolName, SourceLabel, StateRecordId, StreetLine,
    Website, ZipCode,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

use super::*;
use crate::school_address::{CorpusReport, LaneReport, Report};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type ClaimRow = (String, String, String, Option<String>);

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
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Ohio,
        "Springfield High School",
        "springfield high school",
        Some("Springfield"),
    );
    store.append(Table::Schools, &school)?;
    Ok((dir, store))
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
    let evidence = merged_school(&store)?.evidence.len();
    let (second, outcomes) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; second.linked, 0);
    check!(eq; second.already_linked, 1);
    check!(eq; second.backfilled, 0);
    check!(eq; outcomes.len(), 0);
    let merged = merged_school(&store)?;
    check!(eq; merged.postal_addresses.len(), 1);
    check!(eq; merged.evidence.len(), evidence);
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
    check!(eq; second.backfilled, 1);
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
fn refresh_backfills_a_postal_claim_for_an_owned_identity_without_one() -> TestResult {
    let (_dir, store) = store_with_school()?;
    let mut school = merged_school(&store)?;
    school.source_identities.push(SourceIdentity::new(
        SourceNamespace::school_directory("nces-ccd", UsJurisdiction::Ohio),
        "390000000001",
    ));
    store.append(Table::Schools, &school)?;
    let index = DirectoryIndex::build(&[entry(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "1 Main St",
    )?]);
    let (counters, outcomes) = process(
        &store,
        &index,
        &lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01")),
        Mode::Apply,
    )?;
    check!(eq; counters.linked, 0);
    check!(eq; counters.already_linked, 1);
    check!(eq; counters.backfilled, 1);
    check!(eq; outcomes.len(), 0);
    let merged = merged_school(&store)?;
    check!(eq; merged.postal_addresses.len(), 1);
    check!(eq; merged.postal_addresses[0].capture_sha256(), CAPTURE_SHA);
    check!(eq; merged.postal_addresses[0].evidence().observed_on.as_str(), "2026-08-01");
    Ok(())
}

#[test]
fn refresh_retains_a_new_capture_address_beside_the_owned_claim() -> TestResult {
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
    let second_index = DirectoryIndex::build(&[entry(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "2 Second St",
    )?]);
    let (second, outcomes) = process(&store, &second_index, &lanes, Mode::Apply)?;
    check!(eq; second.linked, 0);
    check!(eq; second.already_linked, 1);
    check!(eq; second.backfilled, 1);
    check!(eq; outcomes.len(), 0);
    let merged = merged_school(&store)?;
    check!(eq; merged.postal_addresses.len(), 2);
    check!(merged.postal_addresses.iter().any(|claim| claim
        .address()
        .line1()
        .map(|line| line.as_str())
        == Some("1 Main St")));
    check!(merged.postal_addresses.iter().any(|claim| claim
        .address()
        .line1()
        .map(|line| line.as_str())
        == Some("2 Second St")));
    Ok(())
}

#[test]
fn refresh_retains_a_new_observation_date_on_the_owned_claim() -> TestResult {
    let (_dir, store) = store_with_school()?;
    let index = DirectoryIndex::build(&[entry(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "1 Main St",
    )?]);
    let (first, _) = process(
        &store,
        &index,
        &lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01")),
        Mode::Apply,
    )?;
    check!(eq; first.linked, 1);
    let (second, _) = process(
        &store,
        &index,
        &lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-09-01")),
        Mode::Apply,
    )?;
    check!(eq; second.linked, 0);
    check!(eq; second.already_linked, 1);
    check!(eq; second.backfilled, 1);
    let merged = merged_school(&store)?;
    check!(eq; merged.postal_addresses.len(), 2);
    check!(merged
        .postal_addresses
        .iter()
        .any(|claim| claim.evidence().observed_on.as_str() == "2026-09-01"));
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
                captured: Vec::new(),
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
                captured: Vec::new(),
            },
            LaneReport {
                source: "association:tssaa".to_string(),
                path: "research/sources/tssaa/directory.html".to_string(),
                sha256: CAPTURE_SHA.to_string(),
                entries: 1,
                skipped: 0,
                notes: 0,
                skipped_rows: Vec::new(),
                note_rows: Vec::new(),
                captured: Vec::new(),
            },
            LaneReport {
                source: "milesplit".to_string(),
                path: "research/sources/milesplit/raw.html".to_string(),
                sha256: CAPTURE_SHA.to_string(),
                entries: 1,
                skipped: 0,
                notes: 0,
                skipped_rows: Vec::new(),
                note_rows: Vec::new(),
                captured: Vec::new(),
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
    let lanes = build_lane_evidence(&report, &overrides)?;
    check!(eq; lanes.tokens().count(), 3);
    check!(lanes.tokens().any(|token| token == "state-ed"));
    check!(lanes.tokens().any(|token| token == "association:tssaa"));
    check!(!lanes.tokens().any(|token| token == "milesplit"));
    let lane = lanes
        .lanes("nces-ccd")
        .first()
        .ok_or("missing nces-ccd lane")?;
    check!(eq; lane.url.as_deref(), Some("https://nces.ed.gov/ccd.zip"));
    check!(eq; lane.observed_on.as_deref(), Some("2026-08-01"));
    check!(eq; lane.capture_sha256.as_str(), CAPTURE_SHA);
    check!(eq; lane.generation.as_str(), GENERATION);

    let bare = build_lane_evidence(&report, &Overrides::default())?;
    let lane = bare
        .lanes("nces-ccd")
        .first()
        .ok_or("missing nces-ccd lane")?;
    check!(eq; lane.url, None);
    check!(eq; lane.observed_on, None);
    Ok(())
}

#[test]
fn a_missing_state_is_reported_instead_of_linked() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Ohio,
        "Nowhere School",
        "nowhere school",
        None,
    );
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
    let lanes = lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01"));
    let (dry, _) = process(&store, &index, &lanes, Mode::DryRun)?;
    check!(eq; dry.review, 1);
    check!(eq; dry.review_filed, 0);
    check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?.len(), 0);

    let (counters, outcomes) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; counters.review, 1);
    check!(eq; counters.ambiguous, 1);
    check!(eq; counters.review_filed, 1);
    check!(eq; outcomes.len(), 1);
    check!(eq; outcomes[0].outcome.as_str(), "review");
    check!(eq; outcomes[0].candidates.len(), 2);

    let merged = merged_school(&store)?;
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    check!(eq; cases.len(), 1);
    check!(eq; cases[0].family.as_str(), SCHOOL_IDENTITY_FAMILY);
    check!(eq; cases[0].state, ReviewState::Pending);
    check!(eq; cases[0].subject_id.as_str(), merged.id.as_str());
    check!(cases[0].detail.contains("nces:390000000002"));
    check!(cases[0].detail.contains("nces:390000000003"));
    check!(cases[0].detail.contains("nces-ccd"));

    let (replay, _) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; replay.review, 1);
    check!(eq; replay.review_filed, 0);
    check!(eq; replay.review_present, 1);
    check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?.len(), 1);
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

fn store_with(
    state: UsJurisdiction,
    name: &str,
    city: &str,
) -> Result<(tempfile::TempDir, Store), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let (school, _) = CanonicalSchool::new(state, name, normalize_name(name), Some(city));
    store.append(Table::Schools, &school)?;
    Ok((dir, store))
}

fn state_record_entry(
    id: &str,
    name: &str,
    city: &str,
    state: UsJurisdiction,
    line1: Option<&str>,
    label: SourceLabel,
) -> Result<SchoolDirectoryEntry, Box<dyn std::error::Error>> {
    let line1 = match line1 {
        Some(line1) => Some(StreetLine::parse(line1)?),
        None => None,
    };
    let address = PostalAddress::of(
        line1,
        None,
        Some(CityName::parse(city)?),
        Some(state),
        Some(ZipCode::parse("43000")?),
    )
    .ok_or("address is empty")?;
    Ok(SchoolDirectoryEntry::identified(
        IdentifiedKey::StateRecord {
            state,
            id: StateRecordId::parse(id)?,
        },
        label,
        Some(SchoolName::parse(name)?),
    )
    .with_address(Some(address))
    .with_grades(Some(span(9, 12)?)))
}

fn lane_of(
    token: &str,
    path: &str,
    url: Option<&str>,
    observed_on: Option<&str>,
    captured: Vec<IdentifiedKey>,
) -> (String, LaneEvidence) {
    lane_of_sha(token, path, CAPTURE_SHA, url, observed_on, captured)
}

fn lane_of_sha(
    token: &str,
    path: &str,
    sha: &str,
    url: Option<&str>,
    observed_on: Option<&str>,
    captured: Vec<IdentifiedKey>,
) -> (String, LaneEvidence) {
    (
        token.to_string(),
        LaneEvidence {
            url: url.map(str::to_string),
            observed_on: observed_on.map(str::to_string),
            path: path.to_string(),
            capture_sha256: sha.to_string(),
            generation: GENERATION.to_string(),
            captured,
        },
    )
}

fn lanes_of(lanes: Vec<(String, LaneEvidence)>) -> LaneSet {
    let mut by_provider: BTreeMap<String, Vec<LaneEvidence>> = BTreeMap::new();
    for (token, lane) in lanes {
        by_provider.entry(token).or_default().push(lane);
    }
    LaneSet::of(by_provider)
}

fn lanes(url: Option<&str>, observed_on: Option<&str>) -> LaneSet {
    lanes_of(vec![lane_of(
        "nces-ccd",
        "research/sources/nces-ccd/raw/ccd.csv",
        url,
        observed_on,
        Vec::new(),
    )])
}

fn lane_map(token: &str, url: Option<&str>, observed_on: Option<&str>) -> LaneSet {
    lanes_of(vec![lane_of(
        token,
        &format!("research/sources/{token}/raw/capture.html"),
        url,
        observed_on,
        Vec::new(),
    )])
}

fn state_education(state: UsJurisdiction) -> SourceLabel {
    SourceLabel::StateEducationAgency { state }
}

fn association(state: UsJurisdiction) -> SourceLabel {
    SourceLabel::AthleticAssociation { state }
}

#[test]
fn a_state_education_record_links_and_stamps_its_identity() -> TestResult {
    let (_dir, store) = store_with(UsJurisdiction::NewYork, "Kingston High School", "Kingston")?;
    let index = DirectoryIndex::build(&[state_record_entry(
        "441001",
        "Kingston High School",
        "Kingston",
        UsJurisdiction::NewYork,
        Some("1 Main St"),
        state_education(UsJurisdiction::NewYork),
    )?]);
    let (counters, outcomes) = process(
        &store,
        &index,
        &lane_map(
            "state-ed",
            Some("https://data.nysed.gov/profile/441001"),
            Some("2026-09-01"),
        ),
        Mode::Apply,
    )?;
    check!(eq; counters.scanned, 1);
    check!(eq; counters.linked, 1);
    check!(eq; counters.refused, 0);
    check!(eq; counters.no_match, 0);
    check!(eq; outcomes.len(), 0);

    let merged = merged_school(&store)?;
    check!(eq; merged.postal_addresses.len(), 1);
    let claim = &merged.postal_addresses[0];
    check!(eq;
        claim.owner().namespace,
        SourceNamespace::school_directory("state-ed", UsJurisdiction::NewYork)
    );
    check!(eq; claim.owner().id.as_str(), "441001");
    check!(eq; claim.source_label(), &state_education(UsJurisdiction::NewYork));
    check!(eq; claim.address().line1().map(|line| line.as_str()), Some("1 Main St"));
    check!(eq; claim.evidence().source.id, "state-ed");
    check!(eq; claim.evidence().observed_on.as_str(), "2026-09-01");
    Ok(())
}

#[test]
fn an_unattested_state_record_without_city_agreement_never_links() -> TestResult {
    let (_dir, store) = store_with(UsJurisdiction::NewYork, "Kingston High School", "Kingston")?;
    let index = DirectoryIndex::build(&[state_record_entry(
        "441001",
        "Kingston High School",
        "Albany",
        UsJurisdiction::NewYork,
        Some("1 Main St"),
        state_education(UsJurisdiction::NewYork),
    )?]);
    let (counters, _) = process(
        &store,
        &index,
        &lane_map(
            "state-ed",
            Some("https://data.nysed.gov/profile/441001"),
            Some("2026-09-01"),
        ),
        Mode::Apply,
    )?;
    check!(eq; counters.no_match, 1);
    check!(eq; counters.linked, 0);

    let merged = merged_school(&store)?;
    check!(eq; merged.postal_addresses.len(), 0);
    check!(eq; merged.source_identities.len(), 0);
    Ok(())
}

#[test]
fn an_attested_state_record_is_matched_without_city_agreement() -> TestResult {
    let (_dir, store) = store_with(UsJurisdiction::NewYork, "Kingston High School", "Kingston")?;
    let mut school = merged_school(&store)?;
    school.source_identities.push(SourceIdentity::new(
        SourceNamespace::school_directory("state-ed", UsJurisdiction::NewYork),
        "441001",
    ));
    store.append(Table::Schools, &school)?;
    let index = DirectoryIndex::build(&[state_record_entry(
        "441001",
        "Kingston High School",
        "Albany",
        UsJurisdiction::NewYork,
        Some("1 Main St"),
        state_education(UsJurisdiction::NewYork),
    )?]);
    let (counters, outcomes) = process(
        &store,
        &index,
        &lane_map(
            "state-ed",
            Some("https://data.nysed.gov/profile/441001"),
            Some("2026-09-01"),
        ),
        Mode::Apply,
    )?;
    check!(eq; counters.already_linked, 1);
    check!(eq; counters.no_match, 0);
    check!(eq; outcomes.len(), 0);

    let merged = merged_school(&store)?;
    check!(eq; merged.source_identities.len(), 1);
    check!(eq; counters.backfilled, 1);
    check!(eq; merged.postal_addresses.len(), 1);
    Ok(())
}

#[test]
fn a_city_only_association_entry_attaches_its_identity_without_a_claim() -> TestResult {
    let (_dir, store) = store_with(UsJurisdiction::Tennessee, "Alcoa High School", "Alcoa")?;
    let index = DirectoryIndex::build(&[state_record_entry(
        "3",
        "Alcoa High School",
        "Alcoa",
        UsJurisdiction::Tennessee,
        None,
        association(UsJurisdiction::Tennessee),
    )?]);
    let (counters, outcomes) = process(
        &store,
        &index,
        &lane_map(
            "association:tssaa",
            Some("https://portal.tssaa.org/schools"),
            Some("2026-09-30"),
        ),
        Mode::Apply,
    )?;
    check!(eq; counters.linked, 1);
    check!(eq; counters.refused, 0);
    check!(eq; outcomes.len(), 0);

    let merged = merged_school(&store)?;
    check!(eq; merged.postal_addresses.len(), 0);
    check!(merged.source_identities.iter().any(|identity| {
        identity.namespace == SourceNamespace::association_school("tssaa")
            && identity.id == "3"
            && identity.url.as_deref() == Some("https://portal.tssaa.org/schools")
    }));
    check!(merged.evidence.iter().any(|item| item.source.id == "tssaa"));
    Ok(())
}

#[test]
fn a_record_id_owned_by_another_association_is_refused() -> TestResult {
    let (_dir, store) = store_with(UsJurisdiction::Tennessee, "Alcoa High School", "Alcoa")?;
    let mut school = merged_school(&store)?;
    school.source_identities.push(SourceIdentity::new(
        SourceNamespace::association_school("ihsa"),
        "3",
    ));
    store.append(Table::Schools, &school)?;
    let index = DirectoryIndex::build(&[state_record_entry(
        "3",
        "Alcoa High School",
        "Alcoa",
        UsJurisdiction::Tennessee,
        None,
        association(UsJurisdiction::Tennessee),
    )?]);
    let (counters, outcomes) = process(
        &store,
        &index,
        &lane_map(
            "association:tssaa",
            Some("https://portal.tssaa.org/schools"),
            Some("2026-09-30"),
        ),
        Mode::Apply,
    )?;
    check!(eq; counters.refused, 1);
    check!(eq; counters.linked, 0);
    check!(eq; outcomes.len(), 1);

    let merged = merged_school(&store)?;
    check!(merged
        .source_identities
        .iter()
        .all(|identity| identity.namespace != SourceNamespace::association_school("tssaa")));
    Ok(())
}

#[test]
fn a_replay_of_an_association_link_appends_once() -> TestResult {
    let (_dir, store) = store_with(UsJurisdiction::Tennessee, "Alcoa High School", "Alcoa")?;
    let index = DirectoryIndex::build(&[state_record_entry(
        "3",
        "Alcoa High School",
        "Alcoa",
        UsJurisdiction::Tennessee,
        None,
        association(UsJurisdiction::Tennessee),
    )?]);
    let lanes = lane_map(
        "association:tssaa",
        Some("https://portal.tssaa.org/schools"),
        Some("2026-09-30"),
    );
    let (first, _) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; first.linked, 1);
    let (replay, _) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; replay.already_linked, 1);
    check!(eq; replay.linked, 0);

    let merged = merged_school(&store)?;
    check!(eq; merged.source_identities.len(), 1);
    check!(eq; merged.evidence.len(), 1);
    Ok(())
}

#[test]
fn several_association_lanes_refuse_state_record_links() -> TestResult {
    let (_dir, store) = store_with(UsJurisdiction::Tennessee, "Alcoa High School", "Alcoa")?;
    let index = DirectoryIndex::build(&[state_record_entry(
        "3",
        "Alcoa High School",
        "Alcoa",
        UsJurisdiction::Tennessee,
        None,
        association(UsJurisdiction::Tennessee),
    )?]);
    let lanes = lanes_of(vec![
        lane_of(
            "association:tssaa",
            "research/sources/association:tssaa/raw/capture.html",
            Some("https://portal.tssaa.org/schools"),
            Some("2026-09-30"),
            Vec::new(),
        ),
        lane_of(
            "association:ihsa",
            "research/sources/association:ihsa/raw/capture.html",
            Some("https://www.ihsa.org/schools"),
            Some("2026-09-30"),
            Vec::new(),
        ),
    ]);
    let (counters, outcomes) = process(&store, &index, &lanes, Mode::DryRun)?;
    check!(eq; counters.refused, 1);
    check!(eq; counters.linked, 0);
    check!(outcomes.iter().any(|row| {
        row.detail
            .as_deref()
            .is_some_and(|detail| detail.contains("several association lanes"))
    }));
    Ok(())
}

fn co_op_school(
    name: &str,
    city: &str,
    aliases: &[&str],
) -> Result<CanonicalSchool, Box<dyn std::error::Error>> {
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Tennessee,
        name,
        normalize_name(name),
        Some(city),
    );
    school.co_op = true;
    school.aliases = aliases.iter().map(|alias| (*alias).to_string()).collect();
    Ok(school)
}

fn athlete(
    school: &SchoolId,
    source: SourceIdentity,
) -> Result<CanonicalAthlete, Box<dyn std::error::Error>> {
    let name = "Alex Transfer";
    let grad_year = GradYear::CO2027;
    let gender = Gender::Boys;
    let id = CanonicalAthlete::mint(school, name, grad_year, gender, &source);
    Ok(CanonicalAthlete {
        id,
        canonical_name: normalize_name(name),
        known_names: Vec::new(),
        grad_year,
        school: school.clone(),
        gender,
        sports: Vec::new(),
        observed_grades: Vec::new(),
        published_graduations: Vec::new(),
        public_profile_urls: Vec::new(),
        source: Some(source),
        source_links: Vec::new(),
        evidence: Vec::new(),
        retained_conflicts: Vec::new(),
    })
}

fn transfer_store(
    with_athletes: bool,
) -> Result<(tempfile::TempDir, Store, SchoolId, SchoolId), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let (springfield, springfield_id) = CanonicalSchool::new(
        UsJurisdiction::Tennessee,
        "Springfield High School",
        normalize_name("Springfield High School"),
        Some("Springfield"),
    );
    store.append(Table::Schools, &springfield)?;
    let (shelbyville, shelbyville_id) = CanonicalSchool::new(
        UsJurisdiction::Tennessee,
        "Shelbyville Central High School",
        normalize_name("Shelbyville Central High School"),
        Some("Shelbyville"),
    );
    store.append(Table::Schools, &shelbyville)?;
    if with_athletes {
        let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111");
        store.append(Table::Athletes, &athlete(&springfield_id, source.clone())?)?;
        store.append(Table::Athletes, &athlete(&shelbyville_id, source)?)?;
    }
    Ok((dir, store, springfield_id, shelbyville_id))
}

#[test]
fn a_co_op_school_links_each_independently_qualifying_member() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    store.append(
        Table::Schools,
        &co_op_school(
            "Franklin County Coop",
            "Winchester",
            &[
                "Centennial High School",
                "Page High School",
                "Arab High School",
            ],
        )?,
    )?;
    let index = DirectoryIndex::build(&[
        entry(
            "470000000001",
            "Centennial High School",
            "Franklin",
            UsJurisdiction::Tennessee,
            "1 Centennial Ln",
        )?,
        entry(
            "470000000002",
            "Page High School",
            "Franklin",
            UsJurisdiction::Tennessee,
            "2 Page Ln",
        )?,
        entry(
            "010000000003",
            "Arab High School",
            "Arab",
            UsJurisdiction::Alabama,
            "3 Arab Ln",
        )?,
    ]);
    let (counters, outcomes) = process(
        &store,
        &index,
        &lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01")),
        Mode::Apply,
    )?;
    check!(eq; counters.scanned, 1);
    check!(eq; counters.linked, 2);
    check!(eq; counters.exact_name, 2);
    check!(eq; counters.co_op_members, 3);
    check!(eq; counters.co_op_declined, 1);
    check!(eq; counters.review, 0);
    check!(eq; counters.no_match, 1);
    check!(outcomes.iter().any(|row| row.outcome == "co_op_declined"
        && row.detail.as_deref() == Some("co-op member Arab High School")));
    let mut schools = 0;
    store
        .snapshot()
        .for_each_merged(Table::Schools, |_school: CanonicalSchool| {
            schools += 1;
            Ok(())
        })?;
    check!(eq; schools, 1);
    let merged = merged_school(&store)?;
    check!(eq; merged.co_op, true);
    check!(eq; merged.aliases.len(), 3);
    check!(eq; merged.source_identities.len(), 2);
    check!(eq; merged.postal_addresses.len(), 2);
    let ids: Vec<&str> = merged
        .source_identities
        .iter()
        .map(|identity| identity.id.as_str())
        .collect();
    check!(eq; ids.contains(&"470000000001"), true);
    check!(eq; ids.contains(&"470000000002"), true);
    Ok(())
}

#[test]
fn a_co_op_member_tie_files_one_review_and_attaches_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    store.append(
        Table::Schools,
        &co_op_school(
            "Franklin County Coop",
            "Winchester",
            &["Riverside High School"],
        )?,
    )?;
    let index = DirectoryIndex::build(&[
        entry(
            "470000000010",
            "Riverside High School",
            "Riverside",
            UsJurisdiction::Tennessee,
            "1 River Rd",
        )?,
        entry(
            "470000000011",
            "Riverside High School",
            "Riverside",
            UsJurisdiction::Tennessee,
            "2 River Rd",
        )?,
    ]);
    let (counters, outcomes) = process(
        &store,
        &index,
        &lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01")),
        Mode::Apply,
    )?;
    check!(eq; counters.linked, 0);
    check!(eq; counters.review, 1);
    check!(eq; counters.review_filed, 1);
    check!(eq; counters.co_op_members, 1);
    check!(eq; counters.co_op_declined, 0);
    check!(outcomes.iter().any(|row| row.outcome == "review"
        && row.detail.as_deref() == Some("co-op member Riverside High School")
        && row.candidates.len() == 2));
    let merged = merged_school(&store)?;
    check!(eq; merged.source_identities.len(), 0);
    check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?.len(), 1);
    Ok(())
}

#[test]
fn a_transfer_observation_never_links_schools() -> TestResult {
    let (_plain_dir, plain, _, _) = transfer_store(false)?;
    let (_transfer_dir, transfer, springfield, shelbyville) = transfer_store(true)?;
    let index = DirectoryIndex::build(&[
        entry(
            "470000000020",
            "Springfield High School",
            "Springfield",
            UsJurisdiction::Tennessee,
            "1 Main St",
        )?,
        entry(
            "470000000021",
            "Shelbyville Central High School",
            "Shelbyville",
            UsJurisdiction::Tennessee,
            "2 Depot St",
        )?,
    ]);
    let lanes = lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01"));
    let (plain_counters, plain_outcomes) = process(&plain, &index, &lanes, Mode::Apply)?;
    let (transfer_counters, transfer_outcomes) = process(&transfer, &index, &lanes, Mode::Apply)?;
    check!(eq; transfer_counters.linked, 2);
    check!(eq;
        serde_json::to_value(plain_counters)?,
        serde_json::to_value(transfer_counters)?
    );
    check!(eq;
        serde_json::to_value(&plain_outcomes)?,
        serde_json::to_value(&transfer_outcomes)?
    );
    let mut linked: BTreeMap<String, Vec<String>> = BTreeMap::new();
    transfer
        .snapshot()
        .for_each_merged(Table::Schools, |school: CanonicalSchool| {
            linked.insert(
                school.id.as_str().to_string(),
                school
                    .source_identities
                    .iter()
                    .map(|identity| identity.id.clone())
                    .collect(),
            );
            Ok(())
        })?;
    check!(eq; linked.len(), 2);
    check!(eq;
        linked.get(springfield.as_str()),
        Some(&vec!["470000000020".to_string()])
    );
    check!(eq;
        linked.get(shelbyville.as_str()),
        Some(&vec!["470000000021".to_string()])
    );
    let mut athletes: Vec<String> = Vec::new();
    transfer
        .snapshot()
        .for_each_merged(Table::Athletes, |athlete: CanonicalAthlete| {
            athletes.push(athlete.school.as_str().to_string());
            Ok(())
        })?;
    check!(eq; athletes.len(), 2);
    check!(eq;
        athletes
            .iter()
            .filter(|school| school.as_str() == springfield.as_str())
            .count(),
        1
    );
    check!(eq;
        athletes
            .iter()
            .filter(|school| school.as_str() == shelbyville.as_str())
            .count(),
        1
    );
    Ok(())
}

const ALPHA_SHA: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const BETA_SHA: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn two_school_store() -> Result<(tempfile::TempDir, Store), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    for (name, city) in [("Alpha High School", "Alpha"), ("Beta High School", "Beta")] {
        let (school, _) = CanonicalSchool::new(
            UsJurisdiction::NewYork,
            name,
            normalize_name(name),
            Some(city),
        );
        store.append(Table::Schools, &school)?;
    }
    Ok((dir, store))
}

fn key_of(id: &str) -> Result<IdentifiedKey, Box<dyn std::error::Error>> {
    Ok(IdentifiedKey::StateRecord {
        state: UsJurisdiction::NewYork,
        id: StateRecordId::parse(id)?,
    })
}

fn school_claims(store: &Store) -> TestResult<Vec<ClaimRow>> {
    let mut rows: Vec<ClaimRow> = Vec::new();
    for school in store.scan::<CanonicalSchool>(Table::Schools)? {
        for claim in &school.postal_addresses {
            rows.push((
                school.name.clone(),
                claim.capture_sha256().to_string(),
                claim
                    .evidence()
                    .source
                    .url
                    .as_deref()
                    .map_or(String::new(), str::to_string),
                claim.evidence().note.clone(),
            ));
        }
    }
    rows.sort();
    Ok(rows)
}

#[test]
fn each_state_education_capture_keeps_its_own_provenance() -> TestResult {
    let mut runs = Vec::new();
    for reversed in [false, true] {
        let (_dir, store) = two_school_store()?;
        let alpha = state_record_entry(
            "441001",
            "Alpha High School",
            "Alpha",
            UsJurisdiction::NewYork,
            Some("1 Main St"),
            state_education(UsJurisdiction::NewYork),
        )?;
        let beta = state_record_entry(
            "441002",
            "Beta High School",
            "Beta",
            UsJurisdiction::NewYork,
            Some("2 Side St"),
            state_education(UsJurisdiction::NewYork),
        )?;
        let mut lanes = vec![
            lane_of_sha(
                "state-ed",
                "research/sources/state-ed/raw/alpha.html",
                ALPHA_SHA,
                Some("https://data.nysed.gov/profile/441001"),
                Some("2026-09-01"),
                vec![key_of("441001")?],
            ),
            lane_of_sha(
                "state-ed",
                "research/sources/state-ed/raw/beta.html",
                BETA_SHA,
                Some("https://data.nysed.gov/profile/441002"),
                Some("2026-09-01"),
                vec![key_of("441002")?],
            ),
        ];
        if reversed {
            lanes.reverse();
        }
        let index = DirectoryIndex::build(&[alpha, beta]);
        let (counters, _) = process(&store, &index, &lanes_of(lanes), Mode::Apply)?;
        check!(eq; counters.linked, 2);
        check!(eq; counters.refused, 0);
        check!(eq; counters.evidence_missing, 0);
        let rows = school_claims(&store)?;
        check!(eq; rows.len(), 2);
        let alpha_row = rows
            .iter()
            .find(|row| row.0 == "Alpha High School")
            .ok_or("no alpha claim")?;
        check!(eq; alpha_row.1.as_str(), ALPHA_SHA);
        check!(eq; alpha_row.2.as_str(), "https://data.nysed.gov/profile/441001");
        let alpha_note = alpha_row.3.as_deref().map_or(String::new(), str::to_string);
        check!(alpha_note.contains("alpha.html"));
        check!(!alpha_note.contains("beta.html"));
        let beta_row = rows
            .iter()
            .find(|row| row.0 == "Beta High School")
            .ok_or("no beta claim")?;
        check!(eq; beta_row.1.as_str(), BETA_SHA);
        check!(eq; beta_row.2.as_str(), "https://data.nysed.gov/profile/441002");
        let beta_note = beta_row.3.as_deref().map_or(String::new(), str::to_string);
        check!(beta_note.contains("beta.html"));
        check!(!beta_note.contains("alpha.html"));
        runs.push(rows);
    }
    check!(eq; runs[0], runs[1]);
    Ok(())
}

#[test]
fn two_captures_carrying_one_school_refuse_the_link() -> TestResult {
    let (_dir, store) = store_with(UsJurisdiction::NewYork, "Kingston High School", "Kingston")?;
    let entry = state_record_entry(
        "441001",
        "Kingston High School",
        "Kingston",
        UsJurisdiction::NewYork,
        Some("1 Main St"),
        state_education(UsJurisdiction::NewYork),
    )?;
    let lanes = lanes_of(vec![
        lane_of_sha(
            "state-ed",
            "research/sources/state-ed/raw/march.html",
            ALPHA_SHA,
            Some("https://data.nysed.gov/profile/441001"),
            Some("2026-03-01"),
            vec![key_of("441001")?],
        ),
        lane_of_sha(
            "state-ed",
            "research/sources/state-ed/raw/september.html",
            BETA_SHA,
            Some("https://data.nysed.gov/profile/441001"),
            Some("2026-09-01"),
            vec![key_of("441001")?],
        ),
    ]);
    let index = DirectoryIndex::build(&[entry]);
    let (counters, outcomes) = process(&store, &index, &lanes, Mode::DryRun)?;
    check!(eq; counters.linked, 0);
    check!(eq; counters.refused, 1);
    check!(outcomes.iter().any(|row| row
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("several captures of state-ed")
            && detail.contains("march.html")
            && detail.contains("september.html"))));
    Ok(())
}

#[test]
fn a_capture_selector_names_one_capture_and_an_unknown_one_is_refused() -> TestResult {
    let report = Report {
        manifest_digest: GENERATION.to_string(),
        now: None,
        lanes: vec![
            LaneReport {
                source: "state-ed".to_string(),
                path: "research/sources/state-ed/raw/alpha.html".to_string(),
                sha256: ALPHA_SHA.to_string(),
                entries: 1,
                skipped: 0,
                notes: 0,
                skipped_rows: Vec::new(),
                note_rows: Vec::new(),
                captured: Vec::new(),
            },
            LaneReport {
                source: "state-ed".to_string(),
                path: "research/sources/state-ed/raw/beta.html".to_string(),
                sha256: BETA_SHA.to_string(),
                entries: 1,
                skipped: 0,
                notes: 0,
                skipped_rows: Vec::new(),
                note_rows: Vec::new(),
                captured: Vec::new(),
            },
        ],
        corpus: CorpusReport {
            rows: 2,
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
        urls: BTreeMap::from([
            (
                "state-ed@research/sources/state-ed/raw/alpha.html".to_string(),
                "https://data.nysed.gov/profile/441001".to_string(),
            ),
            (
                "state-ed@research/sources/state-ed/raw/beta.html".to_string(),
                "https://data.nysed.gov/profile/441002".to_string(),
            ),
        ]),
        dates: BTreeMap::from([("state-ed".to_string(), "2026-09-01".to_string())]),
    }
    .validated()?;
    let lanes = build_lane_evidence(&report, &overrides)?;
    let state_ed = lanes.lanes("state-ed");
    check!(eq; state_ed.len(), 2);
    let alpha = state_ed
        .iter()
        .find(|lane| lane.path.ends_with("alpha.html"))
        .ok_or("missing alpha capture")?;
    check!(eq; alpha.url.as_deref(), Some("https://data.nysed.gov/profile/441001"));
    check!(eq; alpha.observed_on.as_deref(), Some("2026-09-01"));
    let beta = state_ed
        .iter()
        .find(|lane| lane.path.ends_with("beta.html"))
        .ok_or("missing beta capture")?;
    check!(eq; beta.url.as_deref(), Some("https://data.nysed.gov/profile/441002"));

    let unknown = Overrides {
        urls: BTreeMap::from([(
            "state-ed@research/sources/state-ed/raw/nope.html".to_string(),
            "https://data.nysed.gov/profile/1".to_string(),
        )]),
        dates: BTreeMap::new(),
    }
    .validated()?;
    let refused = build_lane_evidence(&report, &unknown);
    check!(refused.is_err(), "an unknown capture selector is refused");
    Ok(())
}

#[test]
fn captures_without_per_school_evidence_refuse_to_name_a_bytes_source() -> TestResult {
    let (_dir, store) = store_with(UsJurisdiction::NewYork, "Kingston High School", "Kingston")?;
    let entry = state_record_entry(
        "441001",
        "Kingston High School",
        "Kingston",
        UsJurisdiction::NewYork,
        Some("1 Main St"),
        state_education(UsJurisdiction::NewYork),
    )?;
    let lanes = lanes_of(vec![
        lane_of(
            "state-ed",
            "research/sources/state-ed/raw/march.html",
            Some("https://data.nysed.gov/profile/441001"),
            Some("2026-03-01"),
            Vec::new(),
        ),
        lane_of(
            "state-ed",
            "research/sources/state-ed/raw/september.html",
            Some("https://data.nysed.gov/profile/441001"),
            Some("2026-09-01"),
            Vec::new(),
        ),
    ]);
    let index = DirectoryIndex::build(&[entry]);
    let (counters, outcomes) = process(&store, &index, &lanes, Mode::Apply)?;
    check!(eq; counters.linked, 0);
    check!(eq; counters.refused, 0);
    check!(eq; counters.evidence_missing, 1);
    check!(outcomes.iter().any(|row| row
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("no capture of state-ed carries")
            && detail.contains("march.html")
            && detail.contains("september.html"))));
    check!(eq; school_claims(&store)?.len(), 0);
    Ok(())
}
