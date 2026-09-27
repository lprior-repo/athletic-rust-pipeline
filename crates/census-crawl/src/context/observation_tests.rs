use super::*;
use census_domain::model::{Gender, GradYear, SourceIdentity};
use census_domain::UsJurisdiction;
use census_store::Table;

fn buffered_page<'a>(ctx: &'a AdapterContext<'_>) -> RowBatch<'a> {
    let namespace = SourceNamespace::association_school("synthetic");
    let mut school =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Test School", "test school").0;
    school
        .source_identities
        .push(SourceIdentity::new(namespace.clone(), "school-1"));
    let athlete = CanonicalAthlete::new(
        &school.id,
        "Test Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "12345"),
    );
    let mut page = ctx.write_batch();
    page.append_many(Table::Schools, std::slice::from_ref(&school))
        .expect("school");
    page.append_many(Table::Athletes, std::slice::from_ref(&athlete))
        .expect("athlete");
    page.append_many(
        Table::SourceObservations,
        ctx.school_observation(&namespace, &school).as_slice(),
    )
    .expect("school observation");
    page.append_many(
        Table::SourceObservations,
        &ctx.athlete_observations(&[athlete], [&school]),
    )
    .expect("athlete observation");
    page.journal_done("ownership", "page-1", &serde_json::json!({"mapped":1}))
        .expect("journal");
    page
}

fn scratch() -> (tempfile::TempDir, Store, Fetcher) {
    let directory = tempfile::tempdir().expect("temp directory");
    let store = Store::open(directory.path().join("store")).expect("store");
    let fetcher = Fetcher::new(
        directory.path().join("cache"),
        None,
        std::time::Duration::from_millis(1),
        Default::default(),
        Vec::new(),
    )
    .expect("fetcher");
    (directory, store, fetcher)
}

fn context<'a>(
    store: &'a Store,
    fetcher: &'a Fetcher,
    recording: Option<&'a Recording>,
) -> AdapterContext<'a> {
    AdapterContext {
        store,
        fetcher,
        recording,
        refresh: false,
        school_year: census_domain::model::SchoolYear::new(2026).expect("season"),
        observed_on: "2026-09-26".to_owned(),
    }
}

#[test]
fn abandoned_page_leaves_no_school_athlete_observation_or_journal() {
    let (_directory, store, fetcher) = scratch();
    let ctx = context(&store, &fetcher, None);
    drop(buffered_page(&ctx));
    for table in [Table::Schools, Table::Athletes, Table::SourceObservations] {
        assert_eq!(store.walk_table(table).expect("walk").rows, 0, "{table:?}");
    }
    assert!(store.journal_keys("ownership").expect("journal").is_empty());
    buffered_page(&ctx).commit().expect("commit");
    assert_eq!(store.walk_table(Table::Schools).expect("schools").rows, 1);
    assert_eq!(store.walk_table(Table::Athletes).expect("athletes").rows, 1);
    assert_eq!(
        store
            .walk_table(Table::SourceObservations)
            .expect("observations")
            .rows,
        2
    );
    assert!(store
        .journal_keys("ownership")
        .expect("journal")
        .contains("page-1"));
}

#[test]
fn recording_retains_observations_with_the_page_and_never_writes_the_store() {
    let (_directory, store, fetcher) = scratch();
    let recording = Recording::new();
    let ctx = context(&store, &fetcher, Some(&recording));
    drop(buffered_page(&ctx));
    assert!(recording.drain().is_empty());
    buffered_page(&ctx).commit().expect("record commit");
    let captured = recording.drain();
    let observations: Vec<SourceObservation> = captured
        .rows
        .iter()
        .filter(|batch| batch.table == Table::SourceObservations)
        .flat_map(|batch| batch.rows.iter())
        .map(|value| serde_json::from_value(value.clone()).expect("observation"))
        .collect();
    let [SourceObservation::School(school), SourceObservation::Athlete(athlete)] =
        observations.as_slice()
    else {
        panic!("expected school and athlete observations");
    };
    assert_eq!(school.source_school_id, "school-1");
    assert_eq!(athlete.source_athlete_id, "12345");
    assert_eq!(captured.journal.len(), 1);
    assert_eq!(captured.journal[0].key, "page-1");
    for table in [Table::Schools, Table::Athletes, Table::SourceObservations] {
        assert_eq!(store.walk_table(table).expect("walk").rows, 0, "{table:?}");
    }
    assert!(store.journal_keys("ownership").expect("journal").is_empty());
}
