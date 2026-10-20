#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

#[path = "common/capture_cache.rs"]
mod capture_cache;
#[path = "common/durable_capture.rs"]
mod durable_capture;

#[path = "offline_postal_worksheet/fixtures.rs"]
mod fixtures;
#[path = "offline_postal_worksheet/readback.rs"]
mod readback;

use census_report::{export::ExportDataset, workbook};
use census_store::Store;
use fixtures::{collect_results, join_addresses, publish, seed_source_schools};
use readback::{assert_bests, assert_population, assert_postal};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn retained_wiaa_fixture_keeps_all_cohort_prs_and_blank_addresses_after_unrelated_nces_replay(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    seed_source_schools(&store)?;
    collect_results(&store).await?;
    join_addresses(&store, directory.path())?;
    let path = publish(&store, directory.path())?;
    let dataset = ExportDataset::reopen_frozen(
        &path.parent().ok_or("generation")?.join("frozen-input.json"),
    )?;
    assert_population(&path, &dataset)?;
    assert_postal(&path, &dataset)?;
    assert_bests(&path, &dataset)?;
    workbook::publication::verify_published(&path)?;
    Ok(())
}
