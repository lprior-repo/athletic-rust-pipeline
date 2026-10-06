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
fn a_changed_input_publishes_a_new_generation_holding_the_whole_row_set() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = fixture(&directory)?;

    let first = derive(&store, "index", "2026-09-22")?;
    let published = store.snapshot().derived_generation();
    check!(published > 0, "the pass publishes a derived generation");
    let school = school("Second School", "second-school");
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete(&school, "222"))?;

    let second = derive(&store, "index", "2026-09-23")?;
    check!(eq; second.source_identities,
        first.source_identities + 2,
        "the rebuilt rows cover both schools' athletes");
    check!(
        store.snapshot().derived_generation() > published,
        "a changed input publishes a newer generation"
    );
    check!(eq;
        store.scan::<SourceObjectIdentity>(Table::SourceIdentities)?.len(),
        second.source_identities,
        "the published generation is the pass's whole row set");
    check!(store.integrity()?.ok);
    Ok(())
}

#[test]
fn a_repeated_pass_publishes_nothing_and_leaves_reclaim_nothing() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = fixture(&directory)?;

    let first = derive(&store, "index", "2026-09-22")?;
    let published = store.snapshot().derived_generation();
    let receipts = store.receipt_count()?;
    let expected = store.scan::<SourceObjectIdentity>(Table::SourceIdentities)?;
    store.replace_many::<SourceObjectIdentity>(Table::SourceIdentities, &[])?;
    check!(eq;
        store.scan::<SourceObjectIdentity>(Table::SourceIdentities)?,
        expected,
        "an empty replacement names no row, so a receipt never hides a hole");
    check!(expected
        .iter()
        .any(|row| row.namespace == SourceNamespace::MilesplitAthlete && row.source_id == "111"));

    let second = derive(&store, "index", "2026-09-22")?;
    check!(eq; second.source_identities, first.source_identities);
    check!(eq;
        store.snapshot().derived_generation(),
        published,
        "a pass whose inputs are unchanged publishes no generation");
    check!(eq;
        store.receipt_count()?,
        receipts,
        "a repeat records no second receipt");
    check!(eq; store.scan::<SourceObjectIdentity>(Table::SourceIdentities)?, expected);
    check!(eq;
        store.reclaim_derived_generations(u64::MAX)?.rows,
        0,
        "the abandoned stage of the repeat is already reclaimed");
    check!(store.integrity()?.ok);
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
    store.replace(Table::ReviewCases, &case)?;
    derive(&store, "index", "2026-09-30")?;
    let cases = store.scan::<ReviewCase>(Table::ReviewCases)?;
    check!(eq; cases.iter().find(|row| row.id == case.id), Some(&case));
    Ok(())
}
