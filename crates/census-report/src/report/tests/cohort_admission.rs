use super::{fixture_source, CanonicalAthlete, CanonicalSchool, Derivation, Evidence, Gender};
use super::{GradYear, Scope, Store, Table, UsJurisdiction};
use crate::export::ExportDataset;
use census_domain::model::{
    CanonicalPerformance, Grade, Id, Mark, ObservedGrade, PublishedGraduation, SchoolYear,
    SourceRef,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn athlete(name: &str, year: GradYear) -> CanonicalAthlete {
    let (_, school) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Fixture", "fixture", None);
    let mut row = CanonicalAthlete::new(&school, name, year, Gender::Girls, fixture_source(name));
    row.evidence.push(Evidence::parsed(
        SourceRef::new(
            "published-roster",
            Some(format!("https://source.example/{name}")),
        ),
        "2026-10-02",
    ));
    row
}

fn published(year: GradYear, owner: &str) -> PublishedGraduation {
    PublishedGraduation {
        grad_year: year,
        source: SourceRef::new(
            "published-roster",
            Some(format!("https://source.example/{owner}")),
        ),
    }
}

fn grade(value: u8, date: &str) -> TestResult<ObservedGrade> {
    Ok(ObservedGrade {
        grade: Grade::new(value).ok_or("invalid fixture grade")?,
        school_year: SchoolYear::from_date(date).ok_or("invalid fixture date")?,
        source: SourceRef::new(
            "published-roster",
            Some(format!("https://source.example/{date}")),
        ),
    })
}

fn dataset(athletes: &[CanonicalAthlete]) -> TestResult<ExportDataset> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let (school, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Fixture", "fixture", None);
    store.append(Table::Schools, &school)?;
    for row in athletes {
        store.append(Table::Athletes, row)?;
    }
    Ok(ExportDataset::load(&store)?)
}

fn performance(subject: &CanonicalAthlete, key: &str) -> TestResult<CanonicalPerformance> {
    let mut row = CanonicalPerformance::new(
        census_domain::model::PerformanceIdentity {
            athlete: &subject.id,
            event: &Id::mint("evt", &["fixture"]),
            meet: &Id::mint("meet", &["fixture"]),
            date: "2026-05-01",
            source_key: key,
        },
        census_domain::model::PerformanceResult {
            team: &Id::mint("team", &["fixture"]),
            mark: Mark::Raw("12.34".to_string()),
            wind_mps: None,
            place: None,
        },
    )?;
    row.source_athlete = subject.source.clone();
    row.evidence = subject.evidence.clone();
    Ok(row)
}

#[test]
fn requested_cohort_excludes_stored_year_without_typed_support() -> TestResult {
    let mut unsupported = athlete("Unsupported", GradYear::CO2027);
    unsupported.evidence.push(Evidence::derived(
        SourceRef::id("cohort-query"),
        "2026-10-02",
        "Class of 2027",
    ));
    let input = dataset(std::slice::from_ref(&unsupported))?;
    let requested = Derivation::of(&input, Scope::AllSources, Some(2027));
    check!(eq; requested.athletes(), &[]);
    let archive = Derivation::of(&input, Scope::AllSources, None);
    check!(eq; archive.athletes(), std::slice::from_ref(&unsupported));
    Ok(())
}

#[test]
fn requested_cohort_keeps_direct_year_provenance_without_results_or_invented_grades() -> TestResult
{
    let mut supported = athlete("Direct", GradYear::CO2027);
    supported
        .published_graduations
        .push(published(GradYear::CO2027, "Direct"));
    let input = dataset(std::slice::from_ref(&supported))?;
    let requested = Derivation::of(&input, Scope::AllSources, Some(2027));
    check!(eq; requested.athletes(), std::slice::from_ref(&supported));
    check!(requested.performances().is_empty());
    Ok(())
}

#[test]
fn requested_cohort_keeps_compatible_grades_in_their_published_academic_year() -> TestResult {
    for observation in [grade(11, "2026-05-01")?, grade(12, "2026-09-01")?] {
        let mut supported = athlete("Grade", GradYear::CO2027);
        supported.observed_grades.push(observation);
        let input = dataset(std::slice::from_ref(&supported))?;
        let requested = Derivation::of(&input, Scope::AllSources, Some(2027));
        check!(eq; requested.athletes(), std::slice::from_ref(&supported));
    }
    Ok(())
}

#[test]
fn requested_cohort_excludes_direct_grade_and_uninferable_grade_contradictions() -> TestResult {
    let other_year = GradYear::new(2028).ok_or("invalid fixture graduation year")?;
    let mut supported = athlete("Contradiction", GradYear::CO2027);
    supported
        .published_graduations
        .push(published(GradYear::CO2027, "Contradiction"));
    let mut direct_conflict = supported.clone();
    direct_conflict
        .published_graduations
        .push(published(other_year, "Other"));
    let mut grade_conflict = supported.clone();
    grade_conflict
        .observed_grades
        .push(grade(10, "2026-05-01")?);
    let mut uninferable_grade = supported;
    uninferable_grade
        .observed_grades
        .push(grade(12, "2100-09-01")?);
    for conflicting in [direct_conflict, grade_conflict, uninferable_grade] {
        let mut input = dataset(std::slice::from_ref(&conflicting))?;
        let result = performance(&conflicting, "conflicting-result")?;
        input.performances.push(result.clone());
        let requested = Derivation::of(&input, Scope::AllSources, Some(2027));
        check!(eq; requested.athletes(), &[]);
        check!(requested.performances().is_empty());
        let archive = Derivation::of(&input, Scope::AllSources, None);
        check!(eq; archive.athletes(), std::slice::from_ref(&conflicting));
        check!(eq; archive.performances(), &[&result]);
    }
    Ok(())
}

#[test]
fn requested_cohort_requires_the_stored_year_to_match_supported_evidence() -> TestResult {
    let other_year = GradYear::new(2028).ok_or("invalid fixture graduation year")?;
    let mut off_cohort = athlete("Other Cohort", other_year);
    off_cohort
        .published_graduations
        .push(published(other_year, "Other Cohort"));
    let input = dataset(std::slice::from_ref(&off_cohort))?;
    check!(eq; Derivation::of(&input, Scope::AllSources, Some(2027)).athletes(),
    &[]);
    check!(eq; Derivation::of(&input, Scope::AllSources, Some(2028)).athletes(),
    std::slice::from_ref(&off_cohort));
    Ok(())
}

#[test]
fn accepted_alias_support_admits_the_canonical_member_and_preserves_result_ownership() -> TestResult
{
    let canonical = athlete("Canonical", GradYear::CO2027);
    let mut alias = athlete("Alias", GradYear::CO2027);
    alias
        .published_graduations
        .push(published(GradYear::CO2027, "Alias"));
    let mut input = dataset(&[canonical.clone(), alias.clone()])?;
    input
        .canonical_aliases
        .insert(alias.id.to_string(), canonical.id.to_string());
    let result = performance(&alias, "alias-result")?;
    input.performances.push(result.clone());
    let requested = Derivation::of(&input, Scope::AllSources, Some(2027));
    let retained = requested
        .athletes()
        .first()
        .ok_or("missing supported canonical member")?;
    check!(eq; requested
        .athletes()
        .iter()
        .map(|row| &row.id)
        .collect::<Vec<_>>(),
    vec![&canonical.id]);
    check!(eq; retained.published_graduations, alias.published_graduations);
    check!(eq; retained.observed_grades, Vec::new());
    check!(eq; retained.evidence,
    [canonical.evidence, alias.evidence].concat());
    check!(eq; requested.performances(), &[&result]);
    Ok(())
}

#[test]
fn aliased_conflicts_remain_archived_but_exclude_requested_members() -> TestResult {
    let other_year = GradYear::new(2028).ok_or("invalid fixture graduation year")?;
    let mut canonical = athlete("Canonical", GradYear::CO2027);
    canonical
        .published_graduations
        .push(published(GradYear::CO2027, "Canonical"));
    let mut direct_alias = athlete("Direct Alias", other_year);
    direct_alias
        .published_graduations
        .push(published(other_year, "Direct Alias"));
    let mut grade_alias = athlete("Grade Alias", GradYear::CO2027);
    grade_alias.observed_grades.push(grade(10, "2026-05-01")?);
    for alias in [direct_alias, grade_alias] {
        let mut input = dataset(&[canonical.clone(), alias.clone()])?;
        input
            .canonical_aliases
            .insert(alias.id.to_string(), canonical.id.to_string());
        let result = performance(&alias, "alias-conflict-result")?;
        input.performances.push(result.clone());
        let requested = Derivation::of(&input, Scope::AllSources, Some(2027));
        check!(eq; requested.athletes(), &[]);
        check!(requested.performances().is_empty());
        let archive = Derivation::of(&input, Scope::AllSources, None);
        let retained = archive
            .athletes()
            .first()
            .ok_or("missing archived canonical member")?;
        check!(eq; archive
            .athletes()
            .iter()
            .map(|row| &row.id)
            .collect::<Vec<_>>(),
        vec![&canonical.id]);
        check!(eq; retained.published_graduations,
        [
            canonical.published_graduations.clone(),
            alias.published_graduations.clone()
        ]
        .concat());
        check!(eq; retained.observed_grades, alias.observed_grades);
        check!(eq; retained.evidence,
        [canonical.evidence.clone(), alias.evidence].concat());
        check!(eq; archive.performances(), &[&result]);
    }
    Ok(())
}

#[test]
fn requested_results_exclude_unknown_joins_but_unfiltered_archive_preserves_them() -> TestResult {
    let other_year = GradYear::new(2028).ok_or("invalid fixture graduation year")?;
    let mut admitted = athlete("Admitted", GradYear::CO2027);
    admitted
        .published_graduations
        .push(published(GradYear::CO2027, "Admitted"));
    let mut off_cohort = athlete("Off Cohort", other_year);
    off_cohort
        .published_graduations
        .push(published(other_year, "Off Cohort"));
    let unmatched = athlete("Unmatched", GradYear::CO2027);
    let dangling = athlete("Dangling", GradYear::CO2027);
    let missing = athlete("Absent Target", GradYear::CO2027);
    let mut input = dataset(&[admitted.clone(), off_cohort.clone()])?;
    input
        .canonical_aliases
        .insert(dangling.id.to_string(), missing.id.to_string());
    let admitted_result = performance(&admitted, "admitted")?;
    input.performances = vec![
        admitted_result.clone(),
        performance(&off_cohort, "off-cohort")?,
        performance(&unmatched, "unmatched")?,
        performance(&dangling, "dangling")?,
    ];
    let requested = Derivation::of(&input, Scope::AllSources, Some(2027));
    check!(eq; requested.athletes(), std::slice::from_ref(&admitted));
    check!(eq; requested.performances(), &[&admitted_result]);
    let archive = Derivation::of(&input, Scope::AllSources, None);
    check!(eq; archive.performances(),
    input.performances.iter().collect::<Vec<_>>());
    Ok(())
}
