use super::*;
use crate::bests;
use crate::report::{self, Scope};
use calamine::{open_workbook, Reader, Xlsx};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CoachRole, CompetitionLevel, Evidence, Gender, GradYear,
    Grade, ObservedGrade, ReviewVerdictRecord, SchoolYear, SourceIdentity, SourceNamespace, Sport,
    ATHLETE_IDENTITY_FAMILY, CONTACT_CONFLICT_FAMILY,
};
use census_domain::model::{CanonicalMeet, SourceRef};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use rust_xlsxwriter::Workbook;
use std::path::Path;

const SHEETS: [&str; 7] = [
    "Schools",
    "Meets",
    "Sources",
    "Coverage",
    "Conflicts",
    "Review",
    "Run Metrics",
];

fn meta_workbook(store: &Store, dir: &Path, scope: Scope) -> std::path::PathBuf {
    let dataset = crate::export::ExportDataset::load(store).unwrap();
    let core = report::build_census(
        &report::Derivation::of(&dataset, Scope::Core, None),
        &store.out_dir(),
    );
    let all_sources = report::build_census(
        &report::Derivation::of(&dataset, Scope::AllSources, None),
        &store.out_dir(),
    );
    let bests = bests::build_from_dataset(
        &dataset,
        &bests::Options {
            scope,
            grad_year: Some(2027),
            limit: None,
        },
    );
    let population = report::Derivation::of(&dataset, scope, None);
    let recruiting = report::Derivation::of(&dataset, scope, Some(2027));
    let path = dir.join("meta.xlsx");
    let mut book = Workbook::new();
    write_meta_sheets(
        &mut book,
        &path,
        RunFacts {
            population: &population,
            recruiting: &recruiting,
            store,
            core: &core,
            all_sources: &all_sources,
            bests: &bests,
            school_year: SchoolYear::new(2026).unwrap(),
        },
    )
    .unwrap();
    book.save(&path).unwrap();
    path
}

fn sheet(path: &Path, name: &str) -> Vec<Vec<String>> {
    let mut book: Xlsx<_> = open_workbook(path).unwrap();
    let range = book.worksheet_range(name).unwrap();
    range
        .rows()
        .map(|row| row.iter().map(|cell| cell.to_string()).collect())
        .collect()
}

fn carries(rows: &[Vec<String>], column: usize, value: &str) -> bool {
    rows.iter()
        .any(|row| row.get(column).is_some_and(|cell| cell == value))
}

fn fixture_source(id: &str) -> SourceIdentity {
    SourceIdentity::new(SourceNamespace::Other("fixture".to_string()), id)
}

#[test]
fn an_empty_store_still_writes_every_sheet_with_its_header() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let path = meta_workbook(&store, dir.path(), Scope::Core);

    let book: Xlsx<_> = open_workbook(&path).unwrap();
    let names = book.sheet_names().to_vec();
    assert_eq!(names.len(), SHEETS.len(), "only the meta sheets: {names:?}");
    for expected in SHEETS {
        assert!(
            names.contains(&expected.to_string()),
            "missing sheet {expected} in {names:?}"
        );
    }

    let expectations = [
        ("Schools", "School ID"),
        ("Meets", "Meet ID"),
        ("Sources", "Source"),
        ("Coverage", "Jurisdiction"),
        ("Conflicts", "Family"),
        ("Review", "Family"),
        ("Run Metrics", "Run metric"),
    ];
    for (name, header_text) in expectations {
        let rows = sheet(&path, name);
        let header = rows.first().cloned().unwrap_or_default();
        assert_eq!(
            header.first().map(String::as_str),
            Some(header_text),
            "{name} header row: {header:?}"
        );
    }

    assert_eq!(sheet(&path, "Schools").len(), 1);
    assert_eq!(sheet(&path, "Meets").len(), 1);
    assert_eq!(sheet(&path, "Conflicts").len(), 1);
    assert_eq!(sheet(&path, "Review").len(), 1);

    assert!(sheet(&path, "Sources").len() > 1, "registry rows");

    assert!(sheet(&path, "Coverage").len() > 1, "jurisdiction rows");
}

#[test]
fn the_sheets_render_the_rows_the_store_retains() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let day = "2026-09-21";
    let evidence = Evidence::parsed(SourceRef::new("wiaa_results", None), day);

    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    store.append(Table::Schools, &school).unwrap();

    let (mut twin, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    twin.id = CanonicalSchool::mint(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford legacy");
    let twin_id = twin.id.clone();
    store.append(Table::Schools, &twin).unwrap();

    let (mut orphan, _orphan_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Nowhere", "nowhere");
    orphan.state = None;
    store.append(Table::Schools, &orphan).unwrap();

    let mut conflicted = CanonicalAthlete::new(
        &school_id,
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("julian-aguilera"),
    );
    conflicted.evidence.push(evidence.clone());
    for (grade, year) in [(11_u8, 2025_i16), (10, 2025)] {
        conflicted.observed_grades.push(ObservedGrade {
            grade: Grade::new(grade).unwrap(),
            school_year: SchoolYear::new(year).expect("the fixture season is a school year"),
            source: SourceRef::id("wiaa_results"),
        });
    }
    let conflicted_id = conflicted.id.clone();
    store.append(Table::Athletes, &conflicted).unwrap();

    let mut unverified = CanonicalAthlete::new(
        &school_id,
        "Nora Brandt",
        GradYear::CO2027,
        Gender::Girls,
        fixture_source("nora-brandt"),
    );
    unverified.evidence.push(evidence.clone());
    let unverified_id = unverified.id.clone();
    store.append(Table::Athletes, &unverified).unwrap();
    store
        .append(
            Table::IdentityVerdicts,
            &ReviewVerdictRecord {
                id: "verdict-1".to_string(),
                case_id: "case-1".to_string(),
                subject_id: conflicted_id.to_string(),
                family: "athlete-identity".to_string(),
                kind: "value_proposed".to_string(),
                field: "identity".to_string(),
                value: "same_person".to_string(),
                accepted: true,
                confidence: 91,
                rationale: "matching school and cohort".to_string(),
                reviewer: "test-model".to_string(),
                observed_at: day.to_string(),
                member_ids: Vec::new(),
            },
        )
        .unwrap();

    for name in ["Renata Falk", "Sofia Meier"] {
        let mut conflicted_coach = CanonicalCoach::new(
            &school_id,
            name,
            Some(Sport::OutdoorTrack),
            Gender::Girls,
            CoachRole::HeadCoach,
        );
        conflicted_coach.professional_email = Some(format!(
            "{}@abbotsford.test",
            name.split_whitespace()
                .next()
                .unwrap_or_default()
                .to_lowercase()
        ));
        conflicted_coach.evidence.push(evidence.clone());
        conflicted_coach
            .tenure_evidence
            .push(census_domain::model::CoachTenureEvidence {
                tenure: census_domain::model::CoachTenure::Current {
                    school_year: SchoolYear::new(2026).unwrap(),
                },
                source: SourceRef::new("synthetic_directory", None),
                source_sha256: "a".repeat(64),
                retrieved_at: "2026-09-20T00:00:00Z".into(),
                statement: "Synthetic academic-year appointment".into(),
            });
        store.append(Table::Coaches, &conflicted_coach).unwrap();
    }

    let mut placed = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "WIAA Division 3 State",
        "2026-06-06",
        CompetitionLevel::State,
    );
    placed.evidence.push(evidence.clone());
    placed
        .source_identities
        .push(census_domain::model::SourceIdentity::new(
            SourceNamespace::LegacyAthleticNet {
                kind: "meet".to_string(),
            },
            "1234567",
        ));
    placed
        .source_identities
        .push(census_domain::model::SourceIdentity::new(
            SourceNamespace::TimerMeet {
                provider: "live_results".to_string(),
            },
            "778",
        ));
    let placed_id = placed.id.clone();
    store.append(Table::Meets, &placed).unwrap();

    let unresolved = CanonicalMeet::new(
        None,
        "Holiday Invitational",
        "2026-12-27",
        CompetitionLevel::Unknown,
    );
    let unresolved_id = unresolved.id.clone();
    store.append(Table::Meets, &unresolved).unwrap();

    let path = meta_workbook(&store, dir.path(), Scope::AllSources);

    let meets = sheet(&path, "Meets");
    assert!(carries(&meets, 0, placed_id.as_str()), "{meets:?}");
    assert!(carries(&meets, 4, "WI"), "{meets:?}");
    assert!(carries(&meets, 0, unresolved_id.as_str()), "{meets:?}");
    assert!(carries(&meets, 4, "??"), "{meets:?}");

    let sources = sheet(&path, "Sources");
    for descriptor in census_crawl::descriptors() {
        assert!(
            carries(&sources, 0, descriptor.slug),
            "missing registry row for {}",
            descriptor.slug
        );
    }
    assert!(carries(&sources, 0, "Grade-evidence source"), "{sources:?}");
    assert!(carries(&sources, 1, "wiaa_results"), "{sources:?}");
    assert!(
        carries(&sources, 0, "Meet provider namespace"),
        "{sources:?}"
    );
    assert!(
        carries(&sources, 1, "timer_meet:live_results"),
        "{sources:?}"
    );

    let schools = sheet(&path, "Schools");
    assert_eq!(schools.first().map(Vec::len), Some(12), "{schools:?}");
    assert!(carries(&schools, 0, twin_id.as_str()), "{schools:?}");
    assert!(carries(&schools, 1, "Abbotsford"), "{schools:?}");
    assert!(carries(&schools, 2, "WI"), "{schools:?}");
    assert!(carries(&schools, 2, "??"), "{schools:?}");

    let conflicts = sheet(&path, "Conflicts");
    assert!(
        carries(&conflicts, 0, CONTACT_CONFLICT_FAMILY),
        "the conflicted coaches are surfaced: {conflicts:?}"
    );

    let review = sheet(&path, "Review");
    assert!(
        carries(&review, 0, ATHLETE_IDENTITY_FAMILY),
        "the athlete identity verdict is published for review: {review:?}"
    );
    assert!(
        carries(&review, 2, conflicted_id.as_str()),
        "the verdict names the conflicted subject: {review:?}"
    );
    assert!(
        carries(&review, 2, unverified_id.as_str()),
        "the unverified athlete is surfaced: {review:?}"
    );
    assert!(
        carries(&review, 1, "WI"),
        "the athlete subject resolves its school jurisdiction: {review:?}"
    );
}

#[test]
fn scope_divergence_is_published_in_the_coverage_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let day = "2026-09-21";
    let core_evidence = Evidence::parsed(SourceRef::new("wiaa_results", None), day);
    let other_evidence = Evidence::parsed(SourceRef::new("athleticnet", None), day);

    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    store.append(Table::Schools, &school).unwrap();

    let meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "WIAA Division 3 State",
        "2026-06-06",
        CompetitionLevel::State,
    );
    store.append(Table::Meets, &meet).unwrap();

    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("julian-aguilera"),
    );
    athlete.evidence.push(core_evidence.clone());
    store.append(Table::Athletes, &athlete).unwrap();

    let mut other_athlete = CanonicalAthlete::new(
        &school_id,
        "Maya Okafor",
        GradYear::CO2027,
        Gender::Girls,
        fixture_source("maya-okafor"),
    );
    other_athlete.evidence.push(other_evidence.clone());
    store.append(Table::Athletes, &other_athlete).unwrap();

    let mut coach = CanonicalCoach::new(
        &school_id,
        "Paula Johnson",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    coach.evidence.push(core_evidence);
    store.append(Table::Coaches, &coach).unwrap();

    let mut other_coach = CanonicalCoach::new(
        &school_id,
        "Sam Okafor",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::AssistantCoach,
    );
    other_coach.evidence.push(other_evidence);
    store.append(Table::Coaches, &other_coach).unwrap();

    let core_path = meta_workbook(&store, dir.path(), Scope::Core);
    let all_sources_path = meta_workbook(&store, dir.path(), Scope::AllSources);

    for (path, expected_core, expected_all) in [(core_path, "1", "2"), (all_sources_path, "1", "2")]
    {
        let metrics = sheet(&path, "Run Metrics");
        let athletes = metrics
            .iter()
            .find(|row| row.first().map(String::as_str) == Some("Athletes") && row.len() > 2)
            .unwrap_or_else(|| panic!("the run metrics publish the athlete counters on {path:?}"));
        assert_eq!(
            athletes.get(1).map(String::as_str),
            Some(expected_core),
            "core athletes on {path:?}"
        );
        assert_eq!(
            athletes.get(3).map(String::as_str),
            Some(expected_all),
            "all sources always publish the second athlete on {path:?}"
        );
    }
}
