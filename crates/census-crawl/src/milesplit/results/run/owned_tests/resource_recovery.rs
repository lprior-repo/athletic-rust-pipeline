use super::bounded_buffers::{document, replay_once, seed_set};
use super::*;
use crate::milesplit::results::ResultSetOptions;

#[test]
fn oversized_source_row_is_rejected_before_projection_growth_and_later_rows_survive() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let mut source = document(725218, 1, 3)?;
            source["data"][1]["unprojectedProviderText"] = json!("x".repeat(160_000));
            let denied_id = source["data"][1]["id"]
                .as_str()
                .ok_or("denied ID")?
                .to_string();
            let body = serde_json::to_vec(&source)?;
            seed_owned(&fetcher, &reference, &body)?;
            let options = ResultSetOptions {
                urls: vec![seed_set(&fetcher, 725218, 1266814)?],
            };
            let report =
                crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &options)
                    .await?;
            check!(eq; report.rows, 2);
            check!(report.errors > 0);
            check!(eq; report.disposition, crate::CollectionDisposition::Partial);
            check!(eq; report.unfinished, vec![reference.url.clone()]);
            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq; performances.len(), 2);
            check!(performances
                .iter()
                .all(|row| row.source_key != format!("milesplit_result:{denied_id}")));
            let receipts = store.journal_payloads(crate::milesplit::RESULT_SET_PHASE)?;
            let rejected = receipts
                .iter()
                .find(|row| row["disposition"] == "resource_limit" && row["locator"] == "data[1]")
                .ok_or("unfinished original locator")?;
            check!(eq; rejected["unfinished"], true);
            check!(
                rejected["requested_bytes"]
                    .as_u64()
                    .ok_or("requested bytes")?
                    > rejected["byte_limit"].as_u64().ok_or("byte limit")?
            );
            check!(!receipts
                .iter()
                .any(|row| row["disposition"] == "projection_applied"));
            let source_rows = store.journal_payloads(crate::milesplit::OWNED_MEET_PHASE)?;
            check!(source_rows.iter().any(|row| row["locator"] == "data[1]"
                && row["provider"]["unprojectedProviderText"] == "x".repeat(160_000)));
            replay_once(&store, &fetcher, &options, 2).await?;
            Ok(())
        })
}

#[test]
fn a_recording_limit_between_actual_projection_windows_preserves_prefix_and_resumes_once(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (_dir, store, fetcher, reference) = setup()?;
        let source = document(725218, 1, 500)?;
        seed_owned(&fetcher, &reference, &serde_json::to_vec(&source)?)?;
        let options = ResultSetOptions { urls: vec![seed_set(&fetcher, 725218, 1266814)?] };
        let plain = context(&store, &fetcher)?;
        crate::milesplit::owned::read_owned_meet(&plain, &reference).await?;
        let raw = fetcher.get(&reference.url, &plain.fetch_options()).await?;
        super::super::capture::archive_metadata(&plain, &reference, &raw)?;
        let probe = crate::Recording::new();
        let mut probe_ctx = context(&store, &fetcher)?;
        probe_ctx.recording = Some(&probe);
        let projected = crate::milesplit::collect_result_sets(&probe_ctx, &options).await?;
        check!(eq; (projected.rows, projected.errors), (500, 0));
        let probe = probe.drain();
        let admitted = usize::try_from(probe.journal.iter().filter_map(|entry| entry.payload["admitted_bytes"].as_u64()).max().ok_or("actual admitted windows")?)?;
        let window_room = admitted.checked_mul(2).ok_or("projection window room overflow")?;
        let pressure_bytes = (32usize * 1024 * 1024).checked_sub(window_room).and_then(|bytes| bytes.checked_sub(16 * 1024 + 4096)).ok_or("pressure fixture exceeds recording bound")?;
        let recording = crate::Recording::new();
        let mut pressure = crate::recording::RowSink::Record { store: &store, recording: &recording }.write_batch();
        pressure.journal_done("capacity_pressure", "reserved", &json!({"bytes": "x".repeat(pressure_bytes)}))?;
        pressure.commit()?;
        let mut ctx = context(&store, &fetcher)?;
        ctx.recording = Some(&recording);
        let report = crate::milesplit::collect_result_sets(&ctx, &options).await?;
        check!(report.errors > 0);
        check!(eq; report.disposition, crate::CollectionDisposition::Partial);
        check!(eq; report.unfinished, vec![reference.url.clone()]);
        let usage = recording.usage();
        check!(usage.retained_bytes <= 32 * 1024 * 1024);
        check!(usage.work <= 100_000);
        let mut retained = recording.drain();
        retained.journal.retain(|entry| entry.phase != "capacity_pressure");
        let prefix = retained.rows.iter().filter(|batch| batch.table == Table::Performances).flat_map(|batch| &batch.rows).count();
        check!(prefix > 0 && prefix < 500, "must interrupt the actual collector after a committed window: prefix={prefix}, notes={:?}", report.notes);
        check!(retained.journal.iter().any(|entry| entry.phase == crate::milesplit::RESULT_SET_PHASE && entry.payload["disposition"] == "resource_limit" && entry.payload["unfinished"] == true));
        check!(!retained.journal.iter().any(|entry| entry.phase == crate::milesplit::RESULT_SET_PHASE && entry.payload["disposition"] == "projection_applied"));
        apply(&store, &retained)?;
        check!(eq; store.walk_table(Table::Performances)?.rows, u64::try_from(prefix)?);
        let resumed = crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &options).await?;
        check!(eq; (resumed.rows, resumed.errors), (500, 0));
        let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
        check!(eq; performances.len(), 500);
        check!(performances.iter().all(|row| row.source_athlete.as_ref().is_some_and(|source| source.id == "14222592")));
        check!(eq; store.walk_table(Table::Performances)?.rows, 500);
        replay_once(&store, &fetcher, &options, 500).await?;
        Ok(())
    })
}

#[test]
fn decoded_json_growth_is_refused_before_rows_are_allocated_but_raw_capture_remains() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let mut source = document(725218, 1, 1)?;
            source["providerExtension"] = json!(vec![0; 20_001]);
            let body = serde_json::to_vec(&source)?;
            seed_owned(&fetcher, &reference, &body)?;
            seed_metadata(&fetcher, &reference)?;
            let report = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; report.rows, 0);
            check!(report.errors > 0);
            check!(eq; store.walk_table(Table::Performances)?.rows, 0);
            let receipts = store.journal_payloads(crate::milesplit::OWNED_MEET_PHASE)?;
            check!(receipts
                .iter()
                .any(|row| row["verdict"]["disposition"] == "malformed"
                    && row["verdict"]["detail"]
                        .as_str()
                        .is_some_and(|detail| detail.contains("owned array values"))));
            let captures = store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?;
            check!(captures
                .iter()
                .any(|row| row["raw_base64"].as_str().is_some()));
            check!(!store
                .journal_payloads(crate::milesplit::RESULT_SET_PHASE)?
                .iter()
                .any(|row| row["disposition"] == "projection_applied"));
            Ok(())
        })
}

#[test]
fn raw_metadata_allocation_refusal_archives_exact_capture_and_stops_further_intake() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (_dir, store, fetcher, reference) = setup()?;
        seed_owned(&fetcher, &reference, TROY)?;
        let mut raw = FEMALE_RAW.to_vec();
        raw.extend(vec![b'\n'; 20_001]);
        seed(&fetcher, &reference.url, &raw)?;
        let second = ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266815/raw").ok_or("unread reference")?;
        let mut supplied = options(&reference);
        supplied.urls.extend(options(&second).urls);
        let report = crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &supplied).await?;
        check!(eq; report.rows, 0);
        check!(report.errors > 0);
        check!(eq; report.disposition, crate::CollectionDisposition::Partial);
        check!(eq; report.unfinished, supplied.urls.iter().map(|request| request.url.clone()).collect::<Vec<_>>());
        check!(eq; report.from_cache, 2);
        check!(eq; store.walk_table(Table::Performances)?.rows, 0);
        let receipts = store.journal_payloads(crate::milesplit::RESULT_SET_PHASE)?;
        let stopped = receipts.iter().find(|row| row["disposition"] == "resource_limit").ok_or("unfinished metadata")?;
        check!(eq; stopped["unfinished"], true);
        check!(eq; stopped["raw_metadata_capture"]["content_digest"], crate::net::cache::content_digest(&raw));
        check!(stopped["resource_error"].as_str().is_some_and(|detail| detail.contains("raw metadata lines")));
        check!(!receipts.iter().any(|row| row["disposition"] == "projection_applied"));
        check!(report.notes.iter().any(|note| note == "supplied result locators left unread/unfinished after resource admission: 1"));
        Ok(())
    })
}
