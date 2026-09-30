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
fn an_unchanged_store_skips_the_index_stage() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = fixture(&directory)?;

    let first = derive(&store, "index", "2026-09-22")?;
    assert!(first.source_identities > 0, "{first:?}");

    store.replace_many::<SourceObjectIdentity>(Table::SourceIdentities, &[])?;
    let second = derive(&store, "index", "2026-09-23")?;
    assert_eq!(second.source_identities, 0, "the stage did not rebuild");
    assert_eq!(second.conflicts, 0);
    assert_eq!(second.reviews, 0, "the skip derived no review rows");
    assert_eq!(second.coverage, 0);
    assert_eq!(second.snapshots, 1);
    assert_eq!(second.superseded, 0);
    assert_eq!(second.identity_applications, 0);

    let snapshots: Vec<CollectionSnapshot> = store.scan(Table::Snapshots)?;
    assert!(
        snapshots.iter().any(|row| row.id == "index:2026-09-23"),
        "the skipped pass still records its snapshot row"
    );
    Ok(())
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
    assert_eq!(
        second.source_identities,
        first.source_identities + 2,
        "the rebuilt rows cover both schools' athletes"
    );
    Ok(())
}
