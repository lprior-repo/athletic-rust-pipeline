use super::*;
use census_crawl::milesplit::{
    collect_result_sets, ResultSetOptions, ResultSetRef, ResultSetRequest,
};
use census_domain::model::{
    CanonicalPerformance, CanonicalSchool, SchoolYear, SourceIdentity, SourceNamespace,
};
use census_store::Table;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const OWNED: &[u8] =
    include_bytes!("../../../../census-crawl/src/milesplit/owned/fixtures/troy_725218.json");
const RAW: &[u8] = include_bytes!(
    "../../../../census-crawl/tests/fixtures/milesplit/troy_725218_rs1266814_raw_projection.html"
);
use crate::capture_cache as cache;

fn school(name: &str, owner: &str) -> CanonicalSchool {
    let mut school = CanonicalSchool::new(
        UsJurisdiction::Alabama,
        name,
        census_domain::model::normalize_name(name),
        None,
    )
    .0;
    school
        .source_identities
        .push(SourceIdentity::new(SourceNamespace::MilesplitSchool, owner));
    school
}

fn context<'a>(store: &'a Store, fetcher: &'a Fetcher) -> TestResult<AdapterContext<'a>> {
    Ok(AdapterContext {
        store,
        fetcher,
        refresh: false,
        school_year: SchoolYear::new(2025).ok_or("fixture school year")?,
        observed_on: "2026-10-08".to_string(),
        performance_as_of: chrono::NaiveDate::parse_from_str("2026-10-07", "%Y-%m-%d")?,
        recording: None,
    })
}

#[test]
fn partial_results_replay_then_new_exact_binding_applies_only_new_projection_once() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path().join("store"))?;
        let fetcher = Fetcher::new(dir.path().join("http"), None, std::time::Duration::ZERO,
            std::collections::HashMap::new(), vec!["milesplit.com".to_string()])?.with_offline(true);
        let reference = ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266814/raw")
            .ok_or("fixture result reference")?;
        let owned_url = format!("https://al.milesplit.com/api/v1/meets/725218/performances?isMeetPro=0&fields={}",
            census_crawl::milesplit::OWNED_FIELDS);
        cache::seed(fetcher.cache_dir(), &owned_url, OWNED, "2026-10-01T23:44:16Z", &[])?;
        cache::seed(fetcher.cache_dir(), &reference.url, RAW, "2026-09-28T10:11:24Z", &[])?;
        let options = ResultSetOptions { urls: vec![ResultSetRequest { url: reference.url,
            jurisdiction: UsJurisdiction::Alabama }] };
        let first = collect_result_sets(&context(&store, &fetcher)?, &options).await?;
        check!(eq; first.unresolved, Some(UnresolvedCounters { rows: 3, labels: 2 }));
        check!(eq; store.walk_table(Table::Performances)?.rows, 0);
        let captures = store.journal_payloads(census_crawl::milesplit::OWNED_CAPTURE_PHASE)?;
        let observations = store.walk_table(Table::SourceObservations)?;
        let before = store.snapshot().sequence();
        let repeat = collect_result_sets(&context(&store, &fetcher)?, &options).await?;
        check!(eq; repeat.unresolved, first.unresolved);
        check!(eq; store.snapshot().sequence(), before);
        store.append(Table::Schools, &school("Spann provider school", "38332"))?;
        store.append(Table::Schools, &school("Charles", "4912"))?;
        let resolved = collect_result_sets(&context(&store, &fetcher)?, &options).await?;
        let rows = source_rows("milesplit", 1, resolved).map_err(crate::restate_services::tests::sdk_error)?;
        check!(eq; rows.rows, Some(3));
        check!(eq; rows.withheld, Some(0));
        check!(eq; rows.errors, 0);
        check!(eq; store.walk_table(Table::Performances)?.rows, 3);
        check!(eq; store.walk_table(Table::SourceObservations)?, observations);
        check!(eq; store.journal_payloads(census_crawl::milesplit::OWNED_CAPTURE_PHASE)?, captures);
        let marks: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
        check!(eq; marks.iter().map(|mark| mark.source_key.as_str()).collect::<std::collections::BTreeSet<_>>(),
            std::collections::BTreeSet::from(["milesplit_result:201782263", "milesplit_result:201782277", "milesplit_result:201782806"]));
        let completed = store.snapshot().sequence();
        collect_result_sets(&context(&store, &fetcher)?, &options).await?;
        check!(eq; store.snapshot().sequence(), completed);
        check!(eq; store.scan::<CanonicalPerformance>(Table::Performances)?, marks);
        check!(eq; store.walk_table(Table::Performances)?.rows, 3);
        Ok(())
    })
}
