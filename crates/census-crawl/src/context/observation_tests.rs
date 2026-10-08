use super::*;
use census_domain::model::{Gender, GradYear, SourceIdentity};
use census_domain::UsJurisdiction;
use census_store::Table;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn buffered_page<'a>(ctx: &'a AdapterContext<'_>) -> TestResult<RowBatch<'a>> {
    let namespace = SourceNamespace::association_school("synthetic");
    let mut school = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Test School",
        "test school",
        None,
    )
    .0;
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
    page.append_many(Table::Schools, std::slice::from_ref(&school))?;
    page.append_many(Table::Athletes, std::slice::from_ref(&athlete))?;
    page.append_many(
        Table::SourceObservations,
        ctx.school_observation(&namespace, &school).as_slice(),
    )?;
    page.append_many(
        Table::SourceObservations,
        &ctx.athlete_observations(&[athlete], [&school]),
    )?;
    page.journal_done("ownership", "page-1", &serde_json::json!({"mapped":1}))?;
    Ok(page)
}

fn scratch() -> TestResult<(tempfile::TempDir, Store, Fetcher)> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let fetcher = Fetcher::new(
        directory.path().join("cache"),
        None,
        std::time::Duration::from_millis(1),
        Default::default(),
        Vec::new(),
    )?;
    Ok((directory, store, fetcher))
}

fn context<'a>(
    store: &'a Store,
    fetcher: &'a Fetcher,
    recording: Option<&'a Recording>,
) -> TestResult<AdapterContext<'a>> {
    Ok(AdapterContext {
        store,
        fetcher,
        recording,
        refresh: false,
        school_year: census_domain::model::SchoolYear::new(2026).ok_or("2026 season")?,
        observed_on: "2026-09-26".to_owned(),
        performance_as_of: chrono::NaiveDate::parse_from_str("2026-09-26", "%Y-%m-%d")?,
    })
}

#[test]
fn abandoned_page_leaves_no_school_athlete_observation_or_journal() -> TestResult {
    let (_directory, store, fetcher) = scratch()?;
    let ctx = context(&store, &fetcher, None)?;
    drop(buffered_page(&ctx)?);
    for table in [Table::Schools, Table::Athletes, Table::SourceObservations] {
        check!(eq; store.walk_table(table)?.rows, 0, "{table:?}");
    }
    check!(store.journal_keys("ownership")?.is_empty());
    buffered_page(&ctx)?.commit()?;
    check!(eq; store.walk_table(Table::Schools)?.rows, 1);
    check!(eq; store.walk_table(Table::Athletes)?.rows, 1);
    check!(eq;
        store
            .walk_table(Table::SourceObservations)
            ?
            .rows,
        2
    );
    check!(store.journal_keys("ownership")?.contains("page-1"));
    Ok(())
}

#[test]
fn recording_retains_observations_with_the_page_and_never_writes_the_store() -> TestResult {
    let (_directory, store, fetcher) = scratch()?;
    let recording = Recording::new();
    let ctx = context(&store, &fetcher, Some(&recording))?;
    drop(buffered_page(&ctx)?);
    check!(recording.drain().is_empty());
    buffered_page(&ctx)?.commit()?;
    let captured = recording.drain();
    let observations: Vec<SourceObservation> = captured
        .rows
        .iter()
        .filter(|batch| batch.table == Table::SourceObservations)
        .flat_map(|batch| batch.rows.iter())
        .map(|value| serde_json::from_value(value.clone()))
        .collect::<Result<_, _>>()?;
    let [SourceObservation::School(school), SourceObservation::Athlete(athlete)] =
        observations.as_slice()
    else {
        return Err("expected school and athlete observations".into());
    };
    check!(eq; school.source_school_id, "school-1");
    check!(eq; athlete.source_athlete_id, "12345");
    check!(eq; captured.journal.len(), 1);
    check!(eq; captured.journal[0].key, "page-1");
    for table in [Table::Schools, Table::Athletes, Table::SourceObservations] {
        check!(eq; store.walk_table(table)?.rows, 0, "{table:?}");
    }
    check!(store.journal_keys("ownership")?.is_empty());
    Ok(())
}
