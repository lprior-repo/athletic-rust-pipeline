use super::*;

mod artifacts;
mod source;

use artifacts::assert_artifacts;
use source::{source_report, store_with_source_reports};

#[test]
fn hytek_source_to_pr_xlsx_csv_keeps_faster_thousandth_despite_slower_later_date_and_replay(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = store_with_source_reports(
        &directory.path().join("store"),
        &[("10.941", "5/1/2026"), ("10.944", "5/2/2026")],
    )?;
    assert_artifacts(
        &store,
        &directory.path().join("out"),
        ("10.941", 10.941),
        10_941_000_000,
        3,
    )?;
    let dataset = ExportDataset::load(&store)?;
    let selected = selected(&dataset);
    check!(eq; selected.first().ok_or("missing PR")?.meet.date.as_str(), "2026-05-01");
    check!(eq; selected.first().ok_or("missing PR")?.population.marks, 2);
    Ok(())
}

#[test]
fn hytek_equivalent_spellings_keep_later_precision_and_uncontested_best_in_xlsx_csv() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let store = store_with_source_reports(
        &directory.path().join("store"),
        &[("10.94", "5/1/2026"), ("10.9400", "5/2/2026")],
    )?;
    assert_artifacts(
        &store,
        &directory.path().join("out"),
        ("10.9400", 10.94),
        10_940_000_000,
        4,
    )?;
    let dataset = ExportDataset::load(&store)?;
    let rows = selected(&dataset);
    check!(eq; rows.first().ok_or("missing PR")?.meet.date.as_str(), "2026-05-02");
    check!(eq; rows.first().ok_or("missing PR")?.conflicts, Vec::new());
    Ok(())
}

#[test]
fn hytek_source_rejects_nonfinite_and_overflow_without_minting_a_pr() -> TestResult {
    for raw in [
        "NaN",
        "inf",
        "-inf",
        "9223372036.854775808",
        "153722868:16.854775808",
        "10.9410000001",
    ] {
        let parsed = source_report(raw, "5/1/2026")?;
        check!(eq; (parsed.rows_parsed, parsed.rows_skipped), (0, 1), "{raw}");
        check!(eq; parsed.events.first().ok_or("missing rejected event")?.rows, Vec::new());
    }
    Ok(())
}

#[test]
fn hytek_colon_source_to_pr_xlsx_csv_preserves_exact_nanoseconds_and_precision() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = store_with_source_reports(
        &directory.path().join("store"),
        &[
            ("1:00.000000001", "5/1/2026"),
            ("1:00.000000002", "5/2/2026"),
        ],
    )?;
    assert_artifacts(
        &store,
        &directory.path().join("out"),
        ("1:00.000000001", 60.000000001),
        60_000_000_001,
        9,
    )
}
