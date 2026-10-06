use super::*;
use crate::bests;
use crate::report::{self, Scope};
use calamine::{open_workbook, Reader, Xlsx};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CoachRole, CompetitionLevel, Evidence, Gender, GradYear,
    Grade, ObservedGrade, ReviewCase, ReviewState, ReviewVerdictRecord, SchoolYear, SourceIdentity,
    SourceNamespace, Sport, ATHLETE_IDENTITY_FAMILY, CONTACT_CONFLICT_FAMILY,
    SCHOOL_IDENTITY_FAMILY, UNSUPPORTED_GRADUATION_FAMILY,
};
use census_domain::model::{CanonicalMeet, SourceRef};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use rust_xlsxwriter::Workbook;
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SHEETS: [&str; 7] = [
    "Schools",
    "Meets",
    "Sources",
    "Coverage",
    "Conflicts",
    "Review",
    "Run Metrics",
];

fn meta_workbook(store: &Store, dir: &Path, scope: Scope) -> TestResult<std::path::PathBuf> {
    let dataset = crate::export::ExportDataset::load(store)?;
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
            core: &core,
            all_sources: &all_sources,
            bests: &bests,
            school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
        },
    )?;
    book.save(&path)?;
    Ok(path)
}

fn sheet(path: &Path, name: &str) -> TestResult<Vec<Vec<String>>> {
    let mut book: Xlsx<_> = open_workbook(path)?;
    let range = book.worksheet_range(name)?;
    Ok(range
        .rows()
        .map(|row| row.iter().map(|cell| cell.to_string()).collect())
        .collect())
}

fn carries(rows: &[Vec<String>], column: usize, value: &str) -> bool {
    rows.iter()
        .any(|row| row.get(column).is_some_and(|cell| cell == value))
}

fn fixture_source(id: &str) -> SourceIdentity {
    SourceIdentity::new(SourceNamespace::Other("fixture".to_string()), id)
}

#[test]
fn an_empty_store_still_writes_every_sheet_with_its_header() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let path = meta_workbook(&store, dir.path(), Scope::Core)?;

    let book: Xlsx<_> = open_workbook(&path)?;
    let names = book.sheet_names().to_vec();
    check!(eq; names.len(), SHEETS.len(), "only the meta sheets: {names:?}");
    for expected in SHEETS {
        check!(
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
        let rows = sheet(&path, name)?;
        let header = rows
            .first()
            .cloned()
            .map_or(Default::default(), core::convert::identity);
        check!(eq; header.first().map(String::as_str),
        Some(header_text),
        "{name} header row: {header:?}");
    }

    check!(eq; sheet(&path, "Schools")?.len(), 1);
    check!(eq; sheet(&path, "Meets")?.len(), 1);
    check!(eq; sheet(&path, "Conflicts")?.len(), 1);
    check!(eq; sheet(&path, "Review")?.len(), 1);

    check!(sheet(&path, "Sources")?.len() > 1, "registry rows");

    check!(sheet(&path, "Coverage")?.len() > 1, "jurisdiction rows");
    Ok(())
}

#[test]
fn the_sheets_render_the_rows_the_store_retains() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let day = "2026-09-21";
    let evidence = Evidence::parsed(SourceRef::new("wiaa_results", None), day);

    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    store.append(Table::Schools, &school)?;

    let (mut twin, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    twin.id = CanonicalSchool::mint(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford legacy");
    let twin_id = twin.id.clone();
    store.append(Table::Schools, &twin)?;

    let (mut orphan, _orphan_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Nowhere", "nowhere");
    orphan.state = None;
    store.append(Table::Schools, &orphan)?;

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
            grade: Grade::new(grade).ok_or("invalid fixture grade")?,
            school_year: SchoolYear::new(year).ok_or("invalid fixture season")?,
            source: SourceRef::id("wiaa_results"),
        });
    }
    let conflicted_id = conflicted.id.clone();
    store.append(Table::Athletes, &conflicted)?;

    let mut unverified = CanonicalAthlete::new(
        &school_id,
        "Nora Brandt",
        GradYear::CO2027,
        Gender::Girls,
        fixture_source("nora-brandt"),
    );
    unverified.evidence.push(evidence.clone());
    let unverified_id = unverified.id.clone();
    store.append(Table::Athletes, &unverified)?;
    store.replace(
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
    )?;

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
                .map_or(Default::default(), core::convert::identity)
                .to_lowercase()
        ));
        conflicted_coach.evidence.push(evidence.clone());
        conflicted_coach
            .tenure_evidence
            .push(census_domain::model::CoachTenureEvidence {
                tenure: census_domain::model::CoachTenure::Current {
                    school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
                },
                source: SourceRef::new("synthetic_directory", None),
                source_sha256: "a".repeat(64),
                retrieved_at: "2026-09-20T00:00:00Z".into(),
                statement: "Synthetic academic-year appointment".into(),
                claim: Some(census_domain::model::CoachContactClaim {
                    coach: conflicted_coach.id.clone(),
                    school: conflicted_coach.school.clone(),
                    role: conflicted_coach.role,
                    program: census_domain::model::CoachContactProgram::Team {
                        sport: Sport::OutdoorTrack,
                        gender: conflicted_coach.gender,
                    },
                    mailbox: conflicted_coach.professional_email.clone(),
                }),
            });
        store.append(Table::Coaches, &conflicted_coach)?;
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
    store.append(Table::Meets, &placed)?;

    let unresolved = CanonicalMeet::new(
        None,
        "Holiday Invitational",
        "2026-12-27",
        CompetitionLevel::Unknown,
    );
    let unresolved_id = unresolved.id.clone();
    store.append(Table::Meets, &unresolved)?;

    let path = meta_workbook(&store, dir.path(), Scope::AllSources)?;

    let meets = sheet(&path, "Meets")?;
    check!(carries(&meets, 0, placed_id.as_str()), "{meets:?}");
    check!(carries(&meets, 4, "WI"), "{meets:?}");
    check!(carries(&meets, 0, unresolved_id.as_str()), "{meets:?}");
    check!(carries(&meets, 4, "??"), "{meets:?}");

    let sources = sheet(&path, "Sources")?;
    for descriptor in census_crawl::descriptors() {
        check!(
            carries(&sources, 0, descriptor.slug),
            "missing registry row for {}",
            descriptor.slug
        );
    }
    check!(carries(&sources, 0, "Grade-evidence source"), "{sources:?}");
    check!(carries(&sources, 1, "wiaa_results"), "{sources:?}");
    check!(
        carries(&sources, 0, "Meet provider namespace"),
        "{sources:?}"
    );
    check!(
        carries(&sources, 1, "timer_meet:live_results"),
        "{sources:?}"
    );

    let schools = sheet(&path, "Schools")?;
    check!(carries(&schools, 0, twin_id.as_str()), "{schools:?}");
    check!(carries(&schools, 1, "Abbotsford"), "{schools:?}");
    check!(carries(&schools, 2, "WI"), "{schools:?}");
    check!(carries(&schools, 2, "??"), "{schools:?}");

    let conflicts = sheet(&path, "Conflicts")?;
    check!(
        carries(&conflicts, 0, CONTACT_CONFLICT_FAMILY),
        "the conflicted coaches are surfaced: {conflicts:?}"
    );

    let review = sheet(&path, "Review")?;
    check!(
        carries(&review, 0, ATHLETE_IDENTITY_FAMILY),
        "the athlete identity verdict is published for review: {review:?}"
    );
    check!(
        carries(&review, 2, conflicted_id.as_str()),
        "the verdict names the conflicted subject: {review:?}"
    );
    check!(
        carries(&review, 2, unverified_id.as_str()),
        "the unverified athlete is surfaced: {review:?}"
    );
    check!(
        carries(&review, 1, "WI"),
        "the athlete subject resolves its school jurisdiction: {review:?}"
    );
    Ok(())
}
#[test]
fn unsupported_graduation_cases_are_retained_in_review_without_a_canonical_athlete() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;

    let pending = ReviewCase::pending(
        UNSUPPORTED_GRADUATION_FAMILY,
        "tfrrs_in:meet:2040:row:1",
        "Unplaced Runner",
        "Published grade 12 in school year 2040. URL: https://example.test/results.",
    );
    store.replace(Table::ReviewCases, &pending)?;

    let resolved_case = {
        let mut case = ReviewCase::pending(
            UNSUPPORTED_GRADUATION_FAMILY,
            "tfrrs_in:meet:2039:row:1",
            "Resolved Runner",
            "This case was resolved and should not appear.",
        );
        case.state = ReviewState::Resolved;
        case
    };
    store.replace(Table::ReviewCases, &resolved_case)?;

    let path = crate::workbook::build(
        &store,
        &crate::workbook::Options {
            out: Some(dir.path().join("unsupported-cohort.xlsx")),
            school_year: SchoolYear::new(2026),
            ..crate::workbook::Options::default()
        },
    )?;

    let review = sheet(&path, "Review")?;
    check!(
        carries(&review, 0, UNSUPPORTED_GRADUATION_FAMILY),
        "the pending unsupported graduation case is surfaced: {review:?}"
    );
    check!(
        carries(&review, 3, "Unplaced Runner"),
        "the case subject is visible: {review:?}"
    );

    let resolved_rows = review
        .iter()
        .filter(|row| row.iter().any(|cell| cell == "Resolved Runner"))
        .count();
    check!(eq; resolved_rows, 0,
    "a resolved case is excluded from the review queue: {review:?}");

    let athletes = sheet(&path, "Athletes")?;
    let unplaced_athlete_rows = athletes
        .iter()
        .filter(|row| row.iter().any(|cell| cell == "Unplaced Runner"))
        .count();
    check!(eq; unplaced_athlete_rows, 0,
    "an unsupported graduation case does not create a canonical athlete: {athletes:?}");
    Ok(())
}

#[test]
fn school_link_cases_are_retained_in_review_until_a_verdict_answers_them() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;

    let pending = ReviewCase::pending(
        SCHOOL_IDENTITY_FAMILY,
        "wi:madison west high school",
        "Madison West High School",
        "ambiguous between candidates nces:550000000001, nces:550000000002; providers nces-ccd",
    );
    store.replace(Table::ReviewCases, &pending)?;

    let resolved_case = {
        let mut case = ReviewCase::pending(
            SCHOOL_IDENTITY_FAMILY,
            "wi:answered high school",
            "Answered High School",
            "This case was resolved and should not appear.",
        );
        case.state = ReviewState::Resolved;
        case
    };
    store.replace(Table::ReviewCases, &resolved_case)?;

    let path = crate::workbook::build(
        &store,
        &crate::workbook::Options {
            out: Some(dir.path().join("school-links.xlsx")),
            school_year: SchoolYear::new(2026),
            ..crate::workbook::Options::default()
        },
    )?;

    let review = sheet(&path, "Review")?;
    check!(
        carries(&review, 0, SCHOOL_IDENTITY_FAMILY),
        "the pending school link case is surfaced: {review:?}"
    );
    check!(
        carries(&review, 3, "Madison West High School"),
        "the case subject is visible: {review:?}"
    );
    let candidates_seen = review.iter().any(|row| {
        row.get(6)
            .is_some_and(|cell| cell.contains("nces:550000000002"))
    });
    check!(
        candidates_seen,
        "the candidate labels reach the reviewer: {review:?}"
    );

    let resolved_rows = review
        .iter()
        .filter(|row| row.iter().any(|cell| cell == "Answered High School"))
        .count();
    check!(eq; resolved_rows, 0,
    "a resolved case is excluded from the review queue: {review:?}");
    Ok(())
}

#[test]
fn scope_divergence_is_published_in_the_coverage_sheet() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let day = "2026-09-21";
    let core_evidence = Evidence::parsed(SourceRef::new("wiaa_results", None), day);
    let other_evidence = Evidence::parsed(SourceRef::new("athleticnet", None), day);

    let (school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    store.append(Table::Schools, &school)?;

    let meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "WIAA Division 3 State",
        "2026-06-06",
        CompetitionLevel::State,
    );
    store.append(Table::Meets, &meet)?;

    let mut athlete = CanonicalAthlete::new(
        &school_id,
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
        fixture_source("julian-aguilera"),
    );
    athlete.evidence.push(core_evidence.clone());
    store.append(Table::Athletes, &athlete)?;

    let mut other_athlete = CanonicalAthlete::new(
        &school_id,
        "Maya Okafor",
        GradYear::CO2027,
        Gender::Girls,
        fixture_source("maya-okafor"),
    );
    other_athlete.evidence.push(other_evidence.clone());
    store.append(Table::Athletes, &other_athlete)?;

    let mut coach = CanonicalCoach::new(
        &school_id,
        "Paula Johnson",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    coach.evidence.push(core_evidence);
    store.append(Table::Coaches, &coach)?;

    let mut other_coach = CanonicalCoach::new(
        &school_id,
        "Sam Okafor",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::AssistantCoach,
    );
    other_coach.evidence.push(other_evidence);
    store.append(Table::Coaches, &other_coach)?;

    let core_path = meta_workbook(&store, dir.path(), Scope::Core)?;
    let all_sources_path = meta_workbook(&store, dir.path(), Scope::AllSources)?;

    for (path, expected_core, expected_all) in [(core_path, "1", "2"), (all_sources_path, "1", "2")]
    {
        let metrics = sheet(&path, "Run Metrics")?;
        let athletes = metrics
            .iter()
            .find(|row| row.first().map(String::as_str) == Some("Athletes") && row.len() > 2)
            .ok_or_else(|| format!("missing athlete counters in run metrics on {path:?}"))?;
        check!(eq; athletes.get(1).map(String::as_str),
        Some(expected_core),
        "core athletes on {path:?}");
        check!(eq; athletes.get(3).map(String::as_str),
        Some(expected_all),
        "all sources always publish the second athlete on {path:?}");
    }
    Ok(())
}
