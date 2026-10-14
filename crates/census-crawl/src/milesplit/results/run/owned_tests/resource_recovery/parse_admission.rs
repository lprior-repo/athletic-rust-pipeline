use super::*;

#[test]
fn decoded_json_growth_is_refused_before_rows_are_allocated_but_raw_capture_remains() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher, reference) = setup()?;
            let mut source = document(725218, 1, 1)?;
            source["providerExtension"] = json!(vec![0; 20_001]);
            let body = serde_json::to_vec(&source)?;
            seed_owned(&fetcher, &reference, &body)?;
            seed_metadata(&fetcher, &reference)?;
            let second = ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266815/raw")
                .ok_or("later source locator")?;
            let mut supplied = options(&reference);
            supplied.urls.extend(options(&second).urls);
            let report = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?, &supplied,
            ).await?;
            check!(eq; report.rows, 0);
            check!(report.errors > 0);
            check!(eq; report.disposition, crate::CollectionDisposition::Partial);
            check!(eq; report.from_cache, 1);
            check!(eq; report.unfinished, supplied.urls.iter().map(|request| request.url.clone()).collect::<Vec<_>>());
            no_canonical_effects(&store)?;
            let receipts = store.journal_payloads(crate::milesplit::OWNED_MEET_PHASE)?;
            let resource = receipts.iter().find(|row| row["disposition"] == "resource_limit")
                .ok_or("retained typed parser resource gap")?;
            let offset = body.len().checked_sub(3).ok_or("last provider value offset")?;
            let locator = format!("byte[{offset}]");
            check!(eq; body.get(offset), Some(&b'0'));
            check!(eq; resource["unfinished_locator"], locator);
            check!(eq; resource["unfinished"], true);
            check!(eq; resource["capture_available"], true);
            check!(eq; resource["capture"]["content_digest"], crate::net::cache::content_digest(&body));
            check!(eq; resource["resource_error"], format!("owned array values reservation of 20001 exceeds limit 20000 at {locator}"));
            check!(!receipts.iter().any(|row| row.get("verdict").is_some()));
            check!(store.journal_keys(crate::milesplit::OWNED_MEET_PHASE)?.iter()
                .all(|key| key.starts_with("partial/resource-v2/")));
            let path = resource["immutable_body"].as_str().ok_or("immutable original body locator")?;
            check!(eq; std::fs::read(fetcher.cache_dir().join(path))?, body);
            let captures = store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?;
            let mut chunks = captures.iter().filter(|row| row.get("raw_base64").is_some())
                .map(|row| Ok((row["chunk_index"].as_u64().ok_or("original chunk index")?, row)))
                .collect::<TestResult<Vec<_>>>()?;
            chunks.sort_by_key(|(index, _)| *index);
            use base64::{engine::general_purpose::STANDARD, Engine};
            let restored = chunks.into_iter().try_fold(Vec::new(), |mut bytes, (_, row)| -> TestResult<Vec<u8>> {
                bytes.extend(STANDARD.decode(row["raw_base64"].as_str().ok_or("original raw bytes")?)?);
                Ok(bytes)
            })?;
            check!(eq; restored, body);
            let gaps = store.journal_payloads(crate::milesplit::RESULT_SET_PHASE)?;
            let stop = gaps.iter().find(|row| row["disposition"] == "resource_limit")
                .ok_or("collector parser resource stop")?;
            check!(eq; stop["owned_capture"]["content_digest"], crate::net::cache::content_digest(&body));
            check!(eq; stop["raw_metadata_capture"], serde_json::Value::Null);
            check!(!gaps.iter().any(|row| row["disposition"] == "projection_applied"));
            check!(report.notes.iter().any(|note| note == "supplied result locators left unread/unfinished after resource admission: 1"));
            check!(report.notes.iter().any(|note| note.contains(&locator) && note.contains(&reference.url)));
            let phases = [crate::milesplit::OWNED_MEET_PHASE, crate::milesplit::OWNED_CAPTURE_PHASE, crate::milesplit::RESULT_SET_PHASE];
            let retained = phases.iter().map(|phase| journal_snapshot(&store, phase)).collect::<TestResult<Vec<_>>>()?;
            drop(store);
            let store = Store::open(dir.path().join("store"))?;
            let replay = crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &supplied).await?;
            check!(eq; (replay.rows, replay.unfinished), (0, report.unfinished));
            check!(eq; retained, phases.iter().map(|phase| journal_snapshot(&store, phase)).collect::<TestResult<Vec<_>>>()?);
            no_canonical_effects(&store)?;
            Ok(())
        })
}
