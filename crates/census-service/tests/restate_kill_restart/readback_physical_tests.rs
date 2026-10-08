use super::readback_tests::rejection;
use super::{readback, synthetic_corpus, TestResult};
use census_store::Store;

#[test]
fn d_physical_readback_rejects_an_equal_count_replacement() -> TestResult {
    let seeded = synthetic_corpus(2, 2)?;
    let dir = tempfile::tempdir()?;
    let data_dir = dir.path().join("census");
    std::fs::create_dir_all(&data_dir)?;
    let store = Store::open(&data_dir)?;
    seeded.append(&store)?;
    readback::verify_physical_observations(&store, &seeded)?;

    let mut replaced = synthetic_corpus(2, 2)?;
    let athlete = replaced
        .athletes
        .first_mut()
        .ok_or("the fixture corpus holds no athlete")?;
    athlete.canonical_name.push_str(" (replaced)");
    let stats = store.stats()?;
    check!(eq; stats.observations, replaced.appended_rows() as u64,
    "an equal-count replacement must leave the physical observation count unchanged");
    let reason = rejection(
        readback::verify_physical_observations(&store, &replaced),
        "an equal-count replacement of a physical observation",
    )?;
    check!(
        reason.contains("athletes"),
        "the refusal must name the athletes table: {reason}"
    );
    Ok(())
}
