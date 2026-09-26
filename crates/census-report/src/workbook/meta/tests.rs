//! Tests for the operational sheets: the header every sheet must carry, the empty store that must
//! still write valid sheets, the retained rows each queue must render, and the reconciliation block
//! that must not report drift between the row-level tallies and the census.

use super::*;
use crate::bests;
use crate::report::{self, Scope};
use calamine::{open_workbook, Reader, Xlsx};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CoachRole, CompetitionLevel, Evidence, Gender, GradYear,
    Grade, ObservedGrade, ReviewVerdictRecord, SchoolYear, SourceNamespace, Sport,
};
use census_domain::model::{CanonicalMeet, SourceRef};
use census_domain::UsJurisdiction;
use rust_xlsxwriter::Workbook;
use std::path::Path;

/// The seven sheets this module owns, in published order.
const SHEETS: [&str; 7] = [
    "Schools",
    "Meets",
    "Sources",
    "Coverage",
    "Conflicts",
    "Review",
    "Run Metrics",
];

/// Build the meta sheets for one store and return the workbook path.
fn meta_workbook(store: &Store, dir: &Path, scope: Scope) -> std::path::PathBuf {
    let core = report::build_census(store, Scope::Core).unwrap();
    let all_sources = report::build_census(store, Scope::AllSources).unwrap();
    let bests = bests::build(
        store,
        &bests::Options {
            scope,
            grad_year: Some(2027),
            limit: None,
        },
    )
    .unwrap();
    let path = dir.join("meta.xlsx");
    let mut book = Workbook::new();
    write_meta_sheets(
        &mut book,
        &path,
        RunFacts {
            store,
            core: &core,
            all_sources: &all_sources,
            bests: &bests,
            scope,
            perf_population: PerformanceSheetPopulation {
                cohort_year: Some(2027),
                scope,
                cohort_athletes: 1,
                total_rows: 1,
            },
        },
    )
    .unwrap();
    book.save(&path).unwrap();
    path
}

/// One sheet of a written workbook, every written cell as text.
fn sheet(path: &Path, name: &str) -> Vec<Vec<String>> {
    let mut book: Xlsx<_> = open_workbook(path).unwrap();
    let range = book.worksheet_range(name).unwrap();
    range
        .rows()
        .map(|row| row.iter().map(|cell| cell.to_string()).collect())
        .collect()
}

/// True when some row carries `value` in `column`.
fn carries(rows: &[Vec<String>], column: usize, value: &str) -> bool {
    rows.iter()
        .any(|row| row.get(column).is_some_and(|cell| cell == value))
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

    let (mut orphan, orphan_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Nowhere", "nowhere");
    orphan.state = None;
    store.append(Table::Schools, &orphan).unwrap();

    let mut conflicted = CanonicalAthlete::new(
        &school_id,
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
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

    let mut unverified =
        CanonicalAthlete::new(&school_id, "Nora Brandt", GradYear::CO2027, Gender::Girls);
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
    assert_eq!(conflicts.first().map(Vec::len), Some(5), "{conflicts:?}");
    assert!(
        conflicts.iter().skip(1).all(|row| row.len() == 5),
        "{conflicts:?}"
    );
    assert!(carries(&conflicts, 0, "School identity"), "{conflicts:?}");
    assert!(carries(&conflicts, 2, twin_id.as_str()), "{conflicts:?}");
    assert!(carries(&conflicts, 1, "WI"), "{conflicts:?}");
    assert!(
        carries(&conflicts, 0, "Class-of-2027 cohort evidence"),
        "{conflicts:?}"
    );
    assert!(
        carries(&conflicts, 2, conflicted_id.as_str()),
        "{conflicts:?}"
    );
    assert!(
        carries(&conflicts, 0, "Recruiting contact conflict"),
        "{conflicts:?}"
    );
    assert!(carries(&conflicts, 2, school_id.as_str()), "{conflicts:?}");
    assert!(
        conflicts.iter().any(|row| row
            .get(4)
            .is_some_and(|detail| detail.contains("no evidenced order picks one"))),
        "the conflict row says the rows cannot be separated: {conflicts:?}"
    );

    let review = sheet(&path, "Review");
    assert_eq!(review.first().map(Vec::len), Some(7), "{review:?}");
    assert!(
        review.iter().skip(1).all(|row| row.len() == 7),
        "{review:?}"
    );
    assert!(
        review.iter().any(|row| {
            row.first()
                .is_some_and(|family| family == "Athlete identity")
                && row.get(2).is_some_and(|id| id == conflicted_id.as_str())
                && row
                    .get(4)
                    .is_some_and(|answer| answer == "identity=same_person")
                && row.get(5).is_some_and(|confidence| confidence == "91")
                && row
                    .get(6)
                    .is_some_and(|detail| detail == "matching school and cohort")
        }),
        "the model verdict is rendered in the review row's audit columns: {review:?}"
    );
    assert!(carries(&review, 0, "Meet venue unresolved"), "{review:?}");
    assert!(carries(&review, 2, unresolved_id.as_str()), "{review:?}");
    assert!(
        carries(&review, 0, "School jurisdiction unresolved"),
        "{review:?}"
    );
    assert!(carries(&review, 2, orphan_id.as_str()), "{review:?}");
    assert!(
        carries(&review, 0, "Class-of-2027 cohort unverified"),
        "{review:?}"
    );
    assert!(carries(&review, 2, unverified_id.as_str()), "{review:?}");
    let metrics = sheet(&path, "Run Metrics");
    assert!(carries(&metrics, 0, "Reconciled counter"), "{metrics:?}");
    assert!(carries(&metrics, 3, "reconciled"), "{metrics:?}");
    assert!(!carries(&metrics, 3, "DIFFERS"), "{metrics:?}");
    assert!(
        carries(&metrics, 0, "Best-mark rows reduced"),
        "{metrics:?}"
    );
    assert!(carries(&metrics, 0, "Method note"), "{metrics:?}");
    assert!(
        carries(
            &metrics,
            1,
            "core performance publication keeps a row only when at least one core-evidence source remains; non-core-only rows are omitted from core counts and sheets"
        ),
        "{metrics:?}"
    );

    let coverage = sheet(&path, "Coverage");
    assert!(carries(&coverage, 0, "Coverage note"), "{coverage:?}");
    assert!(carries(&coverage, 1, "2027"), "{coverage:?}");
}

#[test]
fn reconciliation_uses_the_published_scope_for_both_workbook_views() {
    let store_dir = tempfile::tempdir().unwrap();
    let store = Store::open(store_dir.path()).unwrap();
    let (school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "AthleticLive High",
        "athleticlive high",
    );
    store.append(Table::Schools, &school).unwrap();

    let mut athlete =
        CanonicalAthlete::new(&school_id, "Mirror Only", GradYear::CO2027, Gender::Boys);
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new("athleticnet", None),
        "2026-09-25",
    ));
    store.append(Table::Athletes, &athlete).unwrap();

    let core_dir = tempfile::tempdir().unwrap();
    let all_sources_dir = tempfile::tempdir().unwrap();
    let core_path = meta_workbook(&store, core_dir.path(), Scope::Core);
    let all_sources_path = meta_workbook(&store, all_sources_dir.path(), Scope::AllSources);

    for (path, expected_rows) in [(core_path, "0"), (all_sources_path, "1")] {
        let metrics = sheet(&path, "Run Metrics");
        let row = metrics
            .iter()
            .find(|row| {
                row.first().is_some_and(|label| label == "Athletes")
                    && row.get(3).is_some_and(|status| status == "reconciled")
            })
            .expect("the reconciliation block has an athlete row");
        assert_eq!(
            row.get(1).map(String::as_str),
            Some(expected_rows),
            "{row:?}"
        );
        assert_eq!(
            row.get(2).map(String::as_str),
            Some(expected_rows),
            "{row:?}"
        );
        assert_eq!(
            row.get(3).map(String::as_str),
            Some("reconciled"),
            "{row:?}"
        );
    }
}
