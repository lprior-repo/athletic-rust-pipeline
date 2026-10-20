use super::*;

const DATE_PREDECESSOR_PHASE: &str = "tfrrs_projection_v4";

#[path = "run_receipts_date_fixture.rs"]
mod fixture;
use fixture::*;

#[test]
fn synthetic_v4_ambiguous_day_completion_cannot_certify_v5_and_retains_historical_performances(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
            let store = Store::open(dir.path().join("store"))?;
            let fetcher = crate::net::Fetcher::new(
                dir.path().join("http"),
                None,
                std::time::Duration::ZERO,
                HashMap::new(),
                Vec::new(),
            )?
            .with_offline(true);
            seed(&fetcher, LIST.as_bytes())?;
            let bindings = schools(&store, LIST)?;
            let ctx = context(&store, &fetcher, &capture)?;
            let ambiguous = LIST.replace("Mar 28, 2026", "Mar 28-29, 2026");
            check!(ne; ambiguous, LIST);
            let (old_key, old_receipt) = synthetic_v4_date_projection(&ctx, &bindings, ambiguous.as_bytes()).await?;
            let captures = [capture_bytes(&fetcher, LIST.as_bytes())?, capture_bytes(&fetcher, ambiguous.as_bytes())?];
            let historical = snapshot(&store, DATE_PREDECESSOR_PHASE)?;
            check!(eq; historical.get(&old_key).ok_or("v4 completion")?["body_sha256"],
                content_digest(ambiguous.as_bytes()));
            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq; performances.len(), 20);
            check!(eq; performances.iter().filter(|row| row.date == "2026-03-28").count(), 4);
            let physical = store.walk_table(Table::Performances)?;
            let all_physical = physical_rows(&store)?;
            let stored_bytes = entity_digest(&store)?;
            let witnesses = snapshot(&store, "crawl_effect_receipts_v1")?;
            let report = collect(&ctx).await?;
            check!(eq; PHASE, "tfrrs_projection_v5");
            check!(eq; report.disposition, crate::CollectionDisposition::Partial);
            check!(eq; (report.errors, report.rows, report.requests, report.from_cache), (1, 16, 0, 1));
            check!(eq; report.unfinished, vec![URL.to_string()]);
            check!(snapshot(&store, PHASE)?.is_empty());
            check!(eq; snapshot(&store, DATE_PREDECESSOR_PHASE)?, historical);
            check!(eq; store.scan::<CanonicalPerformance>(Table::Performances)?, performances);
            check!(eq; store.walk_table(Table::Performances)?, physical);
            check!(eq; physical_rows(&store)?, all_physical);
            check!(eq; entity_digest(&store)?, stored_bytes);
            check!(eq; snapshot(&store, "crawl_effect_receipts_v1")?, witnesses);
            check!(eq; store.receipt(&old_receipt.operation)?, Some(old_receipt.clone()));
            let archive = fetcher.cache_dir().join("archive/bodies")
                .join(format!("{}.body", content_digest(ambiguous.as_bytes())));
            check!(eq; std::fs::read(&archive)?, ambiguous.as_bytes());
            drop(store);
            let store = Store::open(dir.path().join("store"))?;
            let repeated = collect(&context(&store, &fetcher, &capture)?).await?;
            check!(eq; repeated.disposition, crate::CollectionDisposition::Partial);
            check!(eq; (repeated.errors, repeated.rows, repeated.requests, repeated.from_cache), (1, 16, 0, 1));
            check!(eq; repeated.unfinished, vec![URL.to_string()]);
            check!(snapshot(&store, PHASE)?.is_empty());
            check!(eq; snapshot(&store, DATE_PREDECESSOR_PHASE)?, historical);
            check!(eq; store.walk_table(Table::Performances)?, physical);
            check!(eq; store.scan::<CanonicalPerformance>(Table::Performances)?, performances);
            check!(eq; physical_rows(&store)?, all_physical);
            check!(eq; entity_digest(&store)?, stored_bytes);
            check!(eq; snapshot(&store, "crawl_effect_receipts_v1")?, witnesses);
            check!(eq; store.receipt(&old_receipt.operation)?, Some(old_receipt.clone()));
            check!(eq; std::fs::read(archive)?, ambiguous.as_bytes());
            check!(eq; [capture_bytes(&fetcher, LIST.as_bytes())?, capture_bytes(&fetcher, ambiguous.as_bytes())?], captures);
            check!(eq; fetcher.stats().await.physical_requests(), 0);
            Ok(())
        })
}

#[test]
fn synthetic_v4_single_day_completion_is_reinterpreted_as_v5_without_duplicate_physical_rows(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path().join("store"))?;
            let fetcher = crate::net::Fetcher::new(
                dir.path().join("http"),
                None,
                std::time::Duration::ZERO,
                HashMap::new(),
                Vec::new(),
            )?
            .with_offline(true);
            seed(&fetcher, LIST.as_bytes())?;
            let bindings = schools(&store, LIST)?;
            let durable = crate::net::durable_capture_context(&dir.path().join("capture"))?;
            let ctx = context(&store, &fetcher, &durable)?;
            let (old_key, old_receipt) =
                synthetic_v4_date_projection(&ctx, &bindings, LIST.as_bytes()).await?;
            let capture = capture_bytes(&fetcher, LIST.as_bytes())?;
            let historical = snapshot(&store, DATE_PREDECESSOR_PHASE)?;
            let physical = physical_rows(&store)?;
            let stored_bytes = entity_digest(&store)?;
            let witnesses = snapshot(&store, "crawl_effect_receipts_v1")?;
            let report = collect(&ctx).await?;
            check!(eq; (report.errors, report.requests), (0, 0), "{:?}", report.notes);
            check!(eq; (report.rows, report.from_cache), (20, 1));
            check!(eq; report.disposition, crate::CollectionDisposition::Complete);
            check!(report.unfinished.is_empty());
            let current = snapshot(&store, PHASE)?;
            check!(eq; current.len(), 1);
            let new_context = super::super::context(&ctx, &bindings)?;
            let new_key = key(&new_context, URL, &content_digest(LIST.as_bytes()))?;
            check!(ne; old_key, new_key);
            check!(eq; current.get(&new_key).ok_or("v5 completion")?["projection_context"],
                new_context);
            check!(eq; snapshot(&store, DATE_PREDECESSOR_PHASE)?, historical);
            check!(eq; physical_rows(&store)?, physical);
            check!(eq; snapshot(&store, "crawl_effect_receipts_v1")?, witnesses);
            check!(eq; entity_digest(&store)?, stored_bytes);
            check!(eq; store.receipt(&old_receipt.operation)?, Some(old_receipt.clone()));
            drop(store);
            let store = Store::open(dir.path().join("store"))?;
            let repeated = collect(&context(&store, &fetcher, &durable)?).await?;
            check!(eq; (repeated.errors, repeated.requests), (0, 0));
            check!(eq; (repeated.rows, repeated.from_cache), (20, 1));
            check!(eq; repeated.disposition, crate::CollectionDisposition::Complete);
            check!(repeated.unfinished.is_empty());
            check!(eq; physical_rows(&store)?, physical);
            check!(eq; snapshot(&store, "crawl_effect_receipts_v1")?, witnesses);
            check!(eq; snapshot(&store, PHASE)?, current);
            check!(eq; snapshot(&store, DATE_PREDECESSOR_PHASE)?, historical);
            check!(eq; entity_digest(&store)?, stored_bytes);
            check!(eq; store.receipt(&old_receipt.operation)?, Some(old_receipt.clone()));
            check!(eq; fetcher.stats().await.physical_requests(), 0);
            check!(eq; capture_bytes(&fetcher, LIST.as_bytes())?, capture);
            Ok(())
        })
}

#[test]
fn ambiguous_date_replay_retains_captures_history_and_unfinished_url() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
            let store = Store::open(dir.path().join("store"))?;
            let fetcher = crate::net::Fetcher::new(
                dir.path().join("http"),
                None,
                std::time::Duration::ZERO,
                HashMap::new(),
                Vec::new(),
            )?
            .with_offline(true);
            seed(&fetcher, LIST.as_bytes())?;
            schools(&store, LIST)?;
            let ctx = context(&store, &fetcher, &capture)?;
            let initial = collect(&ctx).await?;
            check!(eq; (initial.errors, initial.rows, initial.requests), (0, 20, 0));
            let historical: Vec<census_domain::model::CanonicalPerformance> =
                store.scan(Table::Performances)?;
            check!(eq; historical.len(), 20);
            let receipts = snapshot(&store, PHASE)?;
            let ambiguous = LIST.replace("Mar 28, 2026", "Mar 28-29, 2026");
            seed(&fetcher, ambiguous.as_bytes())?;
            let report = collect(&ctx).await?;
            check!(eq; report.disposition, crate::CollectionDisposition::Partial);
            check!(eq; (report.errors, report.rows, report.requests, report.from_cache), (1, 16, 0, 1));
            check!(eq; report.unfinished, vec![URL.to_string()]);
            let preserved: Vec<census_domain::model::CanonicalPerformance> =
                store.scan(Table::Performances)?;
            check!(eq; preserved, historical);
            check!(eq; snapshot(&store, PHASE)?, receipts);
            for body in [LIST.as_bytes(), ambiguous.as_bytes()] {
                let archived = fetcher.cache_dir().join("archive/bodies")
                    .join(format!("{}.body", content_digest(body)));
                check!(eq; std::fs::read(archived)?, body);
            }
            let stored_bytes = entity_digest(&store)?;
            let physical = physical_rows(&store)?;
            let witnesses = snapshot(&store, "crawl_effect_receipts_v1")?;
            let captures = [capture_bytes(&fetcher, LIST.as_bytes())?, capture_bytes(&fetcher, ambiguous.as_bytes())?];
            drop(store);
            let store = Store::open(dir.path().join("store"))?;
            let repeated = collect(&context(&store, &fetcher, &capture)?).await?;
            check!(eq; repeated.disposition, crate::CollectionDisposition::Partial);
            check!(eq; (repeated.errors, repeated.rows, repeated.requests, repeated.from_cache), (1, 16, 0, 1));
            check!(eq; repeated.unfinished, vec![URL.to_string()]);
            check!(eq; snapshot(&store, PHASE)?, receipts);
            check!(eq; entity_digest(&store)?, stored_bytes);
            check!(eq; physical_rows(&store)?, physical);
            check!(eq; snapshot(&store, "crawl_effect_receipts_v1")?, witnesses);
            check!(eq; [capture_bytes(&fetcher, LIST.as_bytes())?, capture_bytes(&fetcher, ambiguous.as_bytes())?], captures);
            check!(eq; fetcher.stats().await.physical_requests(), 0);
            Ok(())
        })
}

#[test]
fn malformed_date_cells_cannot_complete_projection_before_or_after_reopen() -> TestResult {
    for label in [
        "Mar 28, 2026<div>or Mar 29, 2026</div>",
        "Mar 28, 2026</div><div data-label=\"Meet Date\">Mar 29, 2026",
        "<span>Mar 28, 2026",
        "<span>Mar 28, 2026</b>",
    ] {
        date_cell_refusal_survives_reopen(label)?;
    }
    Ok(())
}

fn date_cell_refusal_survives_reopen(label: &str) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all().build()?.block_on(async {
            let dir = tempfile::tempdir()?;
            let store = Store::open(dir.path().join("store"))?;
            let fetcher = crate::net::Fetcher::new(
                dir.path().join("http"), None, std::time::Duration::ZERO,
                HashMap::new(), Vec::new(),
            )?.with_offline(true);
            let body = LIST.replace("Mar 28, 2026", label);
            seed(&fetcher, body.as_bytes())?;
            schools(&store, &body)?;
            let durable = crate::net::durable_capture_context(&dir.path().join("capture"))?;
            let report = collect(&context(&store, &fetcher, &durable)?).await?;
            check!(eq; report.disposition, crate::CollectionDisposition::Partial);
            check!(eq; (report.errors, report.rows, report.requests, report.from_cache), (1, 16, 0, 1));
            check!(eq; report.unfinished, vec![URL.to_string()]);
            check!(snapshot(&store, PHASE)?.is_empty());
            check!(eq; store.scan::<CanonicalPerformance>(Table::Performances)?.len(), 16);
            let stored_bytes = entity_digest(&store)?;
            let physical = physical_rows(&store)?;
            let witnesses = snapshot(&store, "crawl_effect_receipts_v1")?;
            let capture = capture_bytes(&fetcher, body.as_bytes())?;
            drop(store);
            let store = Store::open(dir.path().join("store"))?;
            let repeated = collect(&context(&store, &fetcher, &durable)?).await?;
            check!(eq; repeated.disposition, crate::CollectionDisposition::Partial);
            check!(eq; (repeated.errors, repeated.rows, repeated.requests, repeated.from_cache), (1, 16, 0, 1));
            check!(eq; repeated.unfinished, vec![URL.to_string()]);
            check!(snapshot(&store, PHASE)?.is_empty());
            check!(eq; entity_digest(&store)?, stored_bytes);
            check!(eq; physical_rows(&store)?, physical);
            check!(eq; snapshot(&store, "crawl_effect_receipts_v1")?, witnesses);
            check!(eq; capture_bytes(&fetcher, body.as_bytes())?, capture);
            check!(eq; fetcher.stats().await.physical_requests(), 0);
            Ok(())
        })
}
