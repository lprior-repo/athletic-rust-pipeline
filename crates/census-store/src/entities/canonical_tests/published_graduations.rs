use super::*;
use census_domain::model::PublishedGraduation;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn subject() -> CanonicalAthlete {
    CanonicalAthlete::new(
        &school(),
        "Published Cohort Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1001"),
    )
}

fn published(year: GradYear, source: &str) -> PublishedGraduation {
    PublishedGraduation {
        grad_year: year,
        source: SourceRef::new(source, Some(format!("https://source.example/{source}"))),
    }
}

fn read_subject(
    store: &Store,
    expected: &CanonicalAthlete,
) -> Result<CanonicalAthlete, Box<dyn std::error::Error>> {
    let rows: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    check!(eq; rows.len(), 1);
    let row = rows.into_iter().next().ok_or("missing persisted athlete")?;
    check!(eq; row, *expected);
    Ok(row)
}

#[test]
fn a_published_graduation_added_to_an_existing_subject_survives_merge_and_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut expected = subject();
    let claim = published(GradYear::CO2027, "owned-api");
    {
        let store = Store::open(directory.path())?;
        store.append_many(Table::Athletes, &[expected.clone()])?;
        check!(eq; read_subject(&store, &expected)?.derived_cohort_confidence(), None);
        expected.published_graduations.push(claim.clone());
        store.append_many(Table::Athletes, &[expected.clone(), expected.clone()])?;
        let retained = read_subject(&store, &expected)?;
        check!(eq; retained.published_graduations, vec![claim]);
        check!(eq; retained.observed_grades, Vec::new());
        check!(eq; retained.derived_cohort_confidence(), Some(Confidence::HIGH));
    }
    let retained = read_subject(&Store::open(directory.path())?, &expected)?;
    check!(eq; retained.derived_cohort_confidence(), Some(Confidence::HIGH));
    Ok(())
}

#[test]
fn merging_retains_conflicting_years_and_same_year_claims_from_distinct_sources_once() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let mut first = subject();
    let initial = published(GradYear::CO2027, "owned-api");
    first.published_graduations.push(initial.clone());
    let matching_other_source = published(GradYear::CO2027, "other-api");
    let conflicting_same_source = published(
        GradYear::new(2028).ok_or("invalid fixture year")?,
        "owned-api",
    );
    let mut incoming = subject();
    incoming.published_graduations = vec![
        initial,
        matching_other_source.clone(),
        conflicting_same_source.clone(),
    ];
    let observation = ObservedGrade {
        grade: Grade::new(10).ok_or("invalid fixture grade")?,
        school_year: SchoolYear::new(2025).ok_or("invalid fixture school year")?,
        source: SourceRef::id("published-roster"),
    };
    incoming.observed_grades.push(observation.clone());
    let mut expected = first.clone();
    expected
        .published_graduations
        .extend([matching_other_source, conflicting_same_source]);
    expected.observed_grades.push(observation);
    {
        let store = Store::open(directory.path())?;
        store.append_many(Table::Athletes, &[first, incoming.clone(), incoming])?;
        let retained = read_subject(&store, &expected)?;
        check!(eq; retained.source, expected.source);
        check!(eq; retained.source_links, expected.source_links);
        check!(eq; retained.grad_year, GradYear::CO2027);
        check!(eq; retained.has_cohort_conflict(), true);
        check!(eq; retained.derived_cohort_confidence(), Some(Confidence::LOW));
    }
    let retained = read_subject(&Store::open(directory.path())?, &expected)?;
    check!(eq; retained.derived_cohort_confidence(), Some(Confidence::LOW));
    Ok(())
}

#[test]
fn a_natural_key_collision_does_not_attach_another_subjects_published_claim() {
    let mut retained = subject();
    let mut other = CanonicalAthlete::new(
        &second_school(),
        "Different Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "2002"),
    );
    other.id = retained.id.clone();
    other
        .published_graduations
        .push(published(GradYear::CO2027, "other-api"));
    retained.merge(other);
    assert_eq!(retained.published_graduations, Vec::new());
    assert_eq!(retained.derived_cohort_confidence(), None);
    assert_eq!(retained.retained_conflicts.len(), 1);
    assert_eq!(
        retained.retained_conflicts[0].family,
        CANONICAL_ID_COLLISION_FAMILY
    );
}
