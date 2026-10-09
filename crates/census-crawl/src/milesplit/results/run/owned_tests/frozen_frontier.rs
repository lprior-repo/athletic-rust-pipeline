use super::*;
use crate::milesplit::results::{frontier::frozen::replay, ResultSetOptions, ResultSetRequest};
use census_domain::UsJurisdiction;

fn requests() -> TestResult<ResultSetOptions> {
    let urls = (0..5000u64)
        .map(|ordinal| {
            let set = 1_266_814u64
                .checked_add(ordinal)
                .ok_or("result set ordinal overflow")?;
            Ok(ResultSetRequest {
                url: format!("https://al.milesplit.com/meets/725218/results/{set}/raw"),
                jurisdiction: UsJurisdiction::Alabama,
            })
        })
        .collect::<TestResult<Vec<_>>>()?;
    Ok(ResultSetOptions { urls })
}

fn pressure(store: &Store) -> TestResult<crate::Recording> {
    let recording = crate::Recording::new();
    let mut batch = crate::recording::RowSink::Record {
        store,
        recording: &recording,
    }
    .write_batch();
    batch.journal_done(
        "frontier_capacity",
        "reserved",
        &json!({"bytes": "x".repeat(32 * 1024 * 1024 - 16 * 1024)}),
    )?;
    batch.commit()?;
    Ok(recording)
}

fn range_locator(report: &crate::AdapterReport) -> TestResult<&str> {
    report
        .unfinished
        .iter()
        .find(|value| value.starts_with("file://"))
        .map(String::as_str)
        .ok_or_else(|| "durable original input locator".into())
}

fn exact_recovered_requests(locator: &str, options: &ResultSetOptions) -> TestResult {
    let mut visited = Vec::new();
    visited.try_reserve_exact(904)?;
    replay::range(locator, |ordinal, request| {
        let original = options
            .urls
            .get(ordinal)
            .ok_or_else(|| CrawlError::Invariant {
                detail: "recovered original ordinal out of bounds".into(),
            })?;
        if original != &request {
            return Err(CrawlError::Invariant {
                detail: "recovered request differs from frozen original".into(),
            });
        }
        visited.push(ordinal);
        Ok(())
    })?;
    check!(eq; visited, (4096..5000).collect::<Vec<_>>());
    Ok(())
}

#[test]
fn collector_resource_stop_preserves_bounded_urls_and_verified_original_request_range() -> TestResult
{
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (_dir, store, fetcher, reference) = setup()?;
        seed_owned(&fetcher, &reference, TROY)?;
        seed_metadata(&fetcher, &reference)?;
        let recording = pressure(&store)?;
        let options = requests()?;
        let mut ctx = context(&store, &fetcher)?;
        ctx.recording = Some(&recording);
        let report = crate::milesplit::collect_result_sets(&ctx, &options).await?;
        check!(eq; report.disposition, crate::CollectionDisposition::Partial);
        check!(eq; report.unfinished.len(), 4097);
        check!(eq; report.unfinished.get(..4096).ok_or("bounded exact prefix")?, options.urls.iter().take(4096).map(|request| request.url.clone()).collect::<Vec<_>>());
        exact_recovered_requests(range_locator(&report)?, &options)?;
        check!(recording.usage().retained_bytes <= 32 * 1024 * 1024);
        check!(eq; store.walk_table(Table::Performances)?.rows, 0);
        ctx.performance_as_of = chrono::NaiveDate::parse_from_str("2030-10-07", "%Y-%m-%d")?;
        let changed = crate::milesplit::collect_result_sets(&ctx, &options).await?;
        check!(ne; range_locator(&changed)?, range_locator(&report)?);
        exact_recovered_requests(range_locator(&changed)?, &options)?;
        Ok(())
    })
}

#[test]
fn unavailable_original_input_archive_is_partial_and_explicitly_not_resumable() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            seed_owned(&fetcher, &reference, TROY)?;
            seed_metadata(&fetcher, &reference)?;
            let blocked = fetcher
                .cache_dir()
                .join("archive")
                .join("milesplit-request-inputs");
            std::fs::create_dir_all(blocked.parent().ok_or("archive directory")?)?;
            std::fs::write(&blocked, b"not a directory")?;
            let recording = pressure(&store)?;
            let mut ctx = context(&store, &fetcher)?;
            ctx.recording = Some(&recording);
            let report = crate::milesplit::collect_result_sets(&ctx, &requests()?).await?;
            check!(eq; report.disposition, crate::CollectionDisposition::Partial);
            check!(!report
                .unfinished
                .iter()
                .any(|value| value.starts_with("file://") || value.starts_with("input://")));
            check!(report.unfinished.iter().any(|value| value.contains(
                "exactly 904 unlisted unfinished occurrences; no resumable input locator"
            )));
            check!(eq; store.walk_table(Table::Performances)?.rows, 0);
            Ok(())
        })
}

#[test]
fn changed_frozen_input_bytes_are_rejected_before_any_request_is_recovered() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            seed_owned(&fetcher, &reference, TROY)?;
            seed_metadata(&fetcher, &reference)?;
            let recording = pressure(&store)?;
            let mut ctx = context(&store, &fetcher)?;
            ctx.recording = Some(&recording);
            let report = crate::milesplit::collect_result_sets(&ctx, &requests()?).await?;
            let locator = range_locator(&report)?;
            let path = url::Url::parse(locator)?
                .to_file_path()
                .map_err(|()| "local manifest")?;
            let manifest: serde_json::Value = serde_json::from_reader(std::fs::File::open(&path)?)?;
            let body = path
                .parent()
                .ok_or("archive parent")?
                .join(manifest["input_file"].as_str().ok_or("original file")?);
            let mut bytes = std::fs::read(&body)?;
            let first = bytes.first_mut().ok_or("original body first byte")?;
            *first = b'{';
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::metadata(&body)?.permissions();
            let mut writable = permissions.clone();
            writable.set_mode(permissions.mode() | 0o200);
            std::fs::set_permissions(&body, writable)?;
            std::fs::write(&body, bytes)?;
            std::fs::set_permissions(&body, permissions)?;
            let mut recovered = 0usize;
            let result = replay::range(locator, |_, _| {
                recovered = recovered.saturating_add(1);
                Ok(())
            });
            check!(matches!(result, Err(CrawlError::Invariant { .. })));
            check!(eq; recovered, 0);
            Ok(())
        })
}
