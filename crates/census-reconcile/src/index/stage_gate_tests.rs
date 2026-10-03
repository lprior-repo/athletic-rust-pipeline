use super::*;
use census_domain::model::{Gender, GradYear, SourceNamespace};
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn school(name: &str, slug: &str) -> CanonicalSchool {
    let mut school = CanonicalSchool::new(UsJurisdiction::Wisconsin, name, slug).0;
    school
        .source_identities
        .push(SourceIdentity::new(SourceNamespace::MilesplitSchool, slug));
    school
}

fn athlete(school: &CanonicalSchool, native_id: &str) -> CanonicalAthlete {
    CanonicalAthlete::new(
        &school.id,
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, native_id),
    )
}

fn fixture(dir: &tempfile::TempDir) -> Result<Store, Box<dyn std::error::Error>> {
    let store = Store::open(dir.path())?;
    let school = school("School", "school");
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete(&school, "111"))?;
    Ok(store)
}

#[test]
fn a_changed_input_reruns_the_index_stage() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = fixture(&directory)?;

    let first = derive(&store, "index", "2026-09-22")?;
    let school = school("Second School", "second-school");
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete(&school, "222"))?;

    store.replace_many::<SourceObjectIdentity>(Table::SourceIdentities, &[])?;
    let second = derive(&store, "index", "2026-09-23")?;
    check!(eq; second.source_identities,
    first.source_identities + 2,
    "the rebuilt rows cover both schools' athletes");
    Ok(())
}

#[test]
fn receipt_does_not_hide_missing_mutable_projection_rows() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = fixture(&directory)?;
    derive(&store, "index", "2026-09-22")?;
    let expected = store.scan::<SourceObjectIdentity>(Table::SourceIdentities)?;
    check!(expected
        .iter()
        .any(|row| row.namespace == SourceNamespace::MilesplitAthlete && row.source_id == "111"));
    store.replace_many::<SourceObjectIdentity>(Table::SourceIdentities, &[])?;
    derive(&store, "index", "2026-09-23")?;
    check!(eq; store.scan::<SourceObjectIdentity>(Table::SourceIdentities)?,
    expected);
    Ok(())
}

#[test]
fn index_preserves_located_unsupported_cohort_review_without_a_canonical_subject() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = fixture(&directory)?;
    let case = ReviewCase::pending(
        census_domain::model::UNSUPPORTED_GRADUATION_FAMILY,
        "tfrrs_in:meet:2040:row:1",
        "Unplaced Runner",
        "Published grade 12 in school year 2040. URL: https://example.test/results. Row: 1.",
    );
    store.append(Table::ReviewCases, &case)?;
    derive(&store, "index", "2026-09-30")?;
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    check!(eq; cases.iter().find(|row| row.id == case.id), Some(&case));
    Ok(())
}
