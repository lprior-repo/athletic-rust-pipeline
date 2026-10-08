use super::bounded_buffers::{document, seed_set};
use super::*;
use crate::milesplit::results::ResultSetOptions;

async fn collect_at(
    store: &Store,
    fetcher: &Fetcher,
    options: &ResultSetOptions,
    date: &str,
) -> TestResult<crate::AdapterReport> {
    let mut ctx = context(store, fetcher)?;
    ctx.performance_as_of = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")?;
    Ok(crate::milesplit::collect_result_sets(&ctx, options).await?)
}

#[test]
fn changed_horizon_recomputes_eligible_projection_without_duplicating_raw_capture_or_facts(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (_dir, store, fetcher, reference) = setup()?;
        seed_owned(&fetcher, &reference, &serde_json::to_vec(&document(725218, 1, 1)?)?)?;
        let options = ResultSetOptions { urls: vec![seed_set(&fetcher, 725218, 1266814)?] };
        let future = collect_at(&store, &fetcher, &options, "2026-03-26").await?;
        check!(eq; future.disposition, crate::CollectionDisposition::Complete);
        check!(future.unfinished.is_empty());
        check!(eq; store.walk_table(Table::Performances)?.rows, 0);
        check!(eq; store.walk_table(Table::SourceObservations)?.rows, 1);
        let raw_capture = store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?;
        let admitted = collect_at(&store, &fetcher, &options, "2026-10-07").await?;
        check!(eq; admitted.disposition, crate::CollectionDisposition::Complete);
        check!(eq; store.walk_table(Table::Performances)?.rows, 1);
        check!(eq; store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?, raw_capture);
        let rows = store.walk_table(Table::Performances)?;
        collect_at(&store, &fetcher, &options, "2026-11-07").await?;
        check!(eq; store.walk_table(Table::Performances)?, rows);
        let receipts = store.journal_payloads(crate::milesplit::RESULT_SET_PHASE)?;
        let horizons = receipts.iter().filter(|row| row["disposition"] == "projection_applied").map(|row| Ok(row["performance_as_of"].as_str().ok_or("dated projection receipt")?)).collect::<TestResult<std::collections::BTreeSet<_>>>()?;
        check!(eq; horizons, std::collections::BTreeSet::from(["2026-03-26", "2026-10-07", "2026-11-07"]));
        Ok(())
    })
}

#[test]
fn invalid_calendar_day_and_malformed_suffix_preserve_original_date_and_unfinished_locator(
) -> TestResult {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    ["2026-02-30", "2026-03-27garbage"]
        .into_iter()
        .try_for_each(|date| runtime.block_on(unfinished_date(date)))
}

async fn unfinished_date(date: &str) -> TestResult {
    let (_dir, store, fetcher, reference) = setup()?;
    seed_owned(
        &fetcher,
        &reference,
        &serde_json::to_vec(&document(725218, 1, 1)?)?,
    )?;
    let raw = std::str::from_utf8(FEMALE_RAW)?.replace("2026-03-27", date);
    seed(&fetcher, &reference.url, raw.as_bytes())?;
    let report =
        crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &options(&reference))
            .await?;
    check!(eq; report.disposition, crate::CollectionDisposition::Partial);
    check!(eq; report.unfinished, vec![reference.url.clone()]);
    check!(eq; store.walk_table(Table::Performances)?.rows, 0);
    check!(eq; store.walk_table(Table::SourceObservations)?.rows, 1);
    let receipts = store.journal_payloads(crate::milesplit::RESULT_SET_PHASE)?;
    check!(!receipts
        .iter()
        .any(|row| row["disposition"] == "projection_applied"));
    check!(receipts
        .iter()
        .any(|row| row["disposition"] == "retained_unresolved"
            && row["date"] == date
            && row["performance_as_of"] == "2026-10-07"));
    Ok(())
}
