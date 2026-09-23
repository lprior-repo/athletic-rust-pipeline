//! Tests for the operational sheets: the header every sheet must carry, the empty store that must
//! still write valid sheets, the retained rows each queue must render, and the reconciliation block
//! that must not report drift between the row-level tallies and the census.

use super::*;
use crate::bests;
use crate::report::{self, Scope};
use calamine::{open_workbook, Reader, Xlsx};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CoachRole, CompetitionLevel, Evidence, Gender, GradYear,
    Grade, ObservedGrade, SchoolYear, SourceNamespace, Sport,
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
fn meta_workbook(store: &Store, dir: &Path) -> std::path::PathBuf {
    let core = report::build_census(store, Scope::Core).unwrap();
    let all_sources = report::build_census(store, Scope::AllSources).unwrap();
    let bests = bests::build(
        store,
        &bests::Options {
            scope: Scope::Core,
            grad_year: Some(2027),
            limit: None,
        },
    )
    .unwrap();
    let path = dir.join("meta.xlsx");
    let mut book = Workbook::new();
    write_meta_sheets(&mut book, &path, store, &core, &all_sources, &bests).unwrap();
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
    let path = meta_workbook(&store, dir.path());

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
        ("Conflicts", "Retained conflicts"),
        ("Review", "Retained review queue"),
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

    // An empty store is an empty census, not a failure: the two row-level sheets carry their header
    // and nothing else, and the queues carry zero findings.
    assert_eq!(sheet(&path, "Schools").len(), 1);
    assert_eq!(sheet(&path, "Meets").len(), 1);
    let conflicts = sheet(&path, "Conflicts");
    assert_eq!(conflicts.len(), 7, "{conflicts:?}");
    for row in conflicts.iter().skip(1).take(4) {
        assert_eq!(row.get(1).map(String::as_str), Some("0"), "{row:?}");
    }
    assert!(
        carries(&conflicts, 0, "Recruiting contact conflict"),
        "the contact family is declared even when no school disagrees: {conflicts:?}"
    );
    let review = sheet(&path, "Review");
    assert_eq!(review.len(), 8, "{review:?}");
    for row in review.iter().skip(1).take(5) {
        assert_eq!(row.get(1).map(String::as_str), Some("0"), "{row:?}");
    }

    // The declarations are not data: the registry and the jurisdiction list are present even when
    // the store holds nothing.
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

    // A second school row whose normalized name collides with the first but whose id was minted from
    // an older key: the shape a pre-compression journal imports, and the one the census counts as a
    // duplicate name.
    let (mut twin, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    twin.id = CanonicalSchool::mint(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford legacy");
    let twin_id = twin.id.clone();
    store.append(Table::Schools, &twin).unwrap();

    // A school no adapter placed in a jurisdiction.
    let (mut orphan, orphan_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Nowhere", "nowhere");
    orphan.state = None;
    store.append(Table::Schools, &orphan).unwrap();

    // One athlete whose own grade observations disagree about the class, and one with no grade
    // evidence at all: a conflict and a review row, both retained.
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

    // Only a personal mailbox was published, so the collection contract drops it on the way out.
    let mut coach = CanonicalCoach::new(
        &school_id,
        "Dana Whitfield",
        Some(Sport::OutdoorTrack),
        Gender::Girls,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some("dana.whitfield@gmail.com".to_string());
    let coach_id = coach.id.clone();
    store.append(Table::Coaches, &coach).unwrap();

    // Two head coaches of the same program and side, both with a professional address, both observed
    // on the same day: nothing evidenced picks one, so the bucket is a retained contact conflict.
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

    // One placed meet that names its Athletic.net counterpart, and one whose venue was never placed.
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

    let path = meta_workbook(&store, dir.path());

    let schools = sheet(&path, "Schools");
    assert!(carries(&schools, 0, school_id.as_str()), "{schools:?}");
    assert!(carries(&schools, 0, orphan_id.as_str()), "{schools:?}");

    let meets = sheet(&path, "Meets");
    assert!(carries(&meets, 0, placed_id.as_str()), "{meets:?}");
    assert!(carries(&meets, 4, "WI"), "{meets:?}");
    assert!(carries(&meets, 0, unresolved_id.as_str()), "{meets:?}");
    assert!(carries(&meets, 4, "??"), "{meets:?}");

    let sources = sheet(&path, "Sources");
    for descriptor in crate::sources::descriptors() {
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

    let conflicts = sheet(&path, "Conflicts");
    assert!(carries(&conflicts, 0, "School identity"), "{conflicts:?}");
    assert!(carries(&conflicts, 1, twin_id.as_str()), "{conflicts:?}");
    assert!(
        carries(&conflicts, 0, "Class-of-2027 cohort evidence"),
        "{conflicts:?}"
    );
    assert!(
        carries(&conflicts, 1, conflicted_id.as_str()),
        "{conflicts:?}"
    );
    assert!(
        carries(&conflicts, 0, "Recruiting contact conflict"),
        "{conflicts:?}"
    );
    assert!(
        carries(&conflicts, 1, school_id.as_str()),
        "the contact conflict names the school as its subject: {conflicts:?}"
    );
    assert!(
        conflicts.iter().any(|row| row
            .get(3)
            .is_some_and(|detail| detail.contains("no evidenced order picks one"))),
        "the conflict row says the rows cannot be separated: {conflicts:?}"
    );

    let review = sheet(&path, "Review");
    assert!(carries(&review, 0, "Meet venue unresolved"), "{review:?}");
    assert!(carries(&review, 1, unresolved_id.as_str()), "{review:?}");
    assert!(
        carries(&review, 0, "School jurisdiction unresolved"),
        "{review:?}"
    );
    assert!(
        carries(&review, 0, "Class-of-2027 cohort unverified"),
        "{review:?}"
    );
    assert!(carries(&review, 1, unverified_id.as_str()), "{review:?}");
    assert!(carries(&review, 0, "Coach mailbox withheld"), "{review:?}");
    assert!(carries(&review, 1, coach_id.as_str()), "{review:?}");

    // Every row-level tally the run-metrics sheet reconciles must agree with the census: a DIFFERS
    // cell is the workbook telling the operator it has drifted from report.json.
    let metrics = sheet(&path, "Run Metrics");
    assert!(carries(&metrics, 0, "Reconciled counter"), "{metrics:?}");
    assert!(carries(&metrics, 3, "reconciled"), "{metrics:?}");
    assert!(!carries(&metrics, 3, "DIFFERS"), "{metrics:?}");
    assert!(
        carries(&metrics, 0, "Best-mark rows reduced"),
        "{metrics:?}"
    );

    // The coverage sheet is the report pass rendered, not recounted here: it names the cohort it was
    // asked for and carries the jurisdictions.
    let coverage = sheet(&path, "Coverage");
    assert!(carries(&coverage, 0, "Coverage note"), "{coverage:?}");
    assert!(carries(&coverage, 1, "2027"), "{coverage:?}");
}
