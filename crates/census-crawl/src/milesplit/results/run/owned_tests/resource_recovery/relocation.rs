use super::*;
use crate::milesplit::{OwnedCaptureRetention, OwnedResourceReceipt};

async fn retained_witness(
    store: &Store,
    fetcher: &Fetcher,
    reference: &ResultSetRef,
    body: &[u8],
    origin: &str,
) -> TestResult<std::path::PathBuf> {
    match crate::milesplit::owned::read_owned_meet(&context(store, fetcher)?, reference).await {
        Err(CrawlError::OwnedParseResource(failure)) => {
            check!(failure.to_string().starts_with(origin));
            match failure.retention() {
                OwnedCaptureRetention::Captured {
                    capture,
                    immutable_body,
                    receipt,
                } => {
                    check!(eq; *receipt, OwnedResourceReceipt::Retained);
                    check!(eq; capture.body.len(), 0);
                    check!(eq; std::fs::read(immutable_body)?, body);
                    check!(eq; capture.content_digest, crate::net::cache::content_digest(body));
                    check!(immutable_body.starts_with(fetcher.cache_dir()));
                    Ok(immutable_body.clone())
                }
                other => Err(format!("current captured witness: {other:?}").into()),
            }
        }
        other => Err(format!("actual parser resource outcome: {other:?}").into()),
    }
}

#[test]
fn actual_collector_resource_replay_survives_physical_store_and_cache_relocation() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (dir, store, fetcher, reference) = setup()?;
        let mut source = document(725218, 1, 1)?;
        source["providerExtension"] = json!(vec![0; 20_001]);
        let body = serde_json::to_vec(&source)?;
        seed_owned(&fetcher, &reference, &body)?;
        seed_metadata(&fetcher, &reference)?;
        let offset = body.len().checked_sub(3).ok_or("original provider byte")?;
        check!(eq; body.get(offset), Some(&b'0'));
        let locator = format!("byte[{offset}]");
        let origin = format!("owned array values reservation of 20001 exceeds limit 20000 at {locator}");
        let second = ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266815/raw")
            .ok_or("later source locator")?;
        let mut supplied = options(&reference);
        supplied.urls.extend(options(&second).urls);
        let report = crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &supplied).await?;
        check!(eq; (report.rows, report.from_cache), (0, 1));
        check!(eq; report.disposition, crate::CollectionDisposition::Partial);
        check!(eq; report.unfinished, supplied.urls.iter().map(|request| request.url.clone()).collect::<Vec<_>>());
        no_canonical_effects(&store)?;
        let original_path = retained_witness(&store, &fetcher, &reference, &body, &origin).await?;
        check!(original_path.is_absolute());
        check!(report.notes.iter().any(|note| note.contains(&original_path.display().to_string())));
        let phases = [crate::milesplit::OWNED_MEET_PHASE, crate::milesplit::OWNED_CAPTURE_PHASE, crate::milesplit::RESULT_SET_PHASE];
        let original = phases.iter().map(|phase| journal_snapshot(&store, phase)).collect::<TestResult<Vec<_>>>()?;
        let owned = journal_snapshot(&store, crate::milesplit::OWNED_MEET_PHASE)?;
        check!(eq; owned.len(), 1);
        let resource = owned.values().next().ok_or("owned resource receipt")?;
        check!(eq; resource["unfinished_locator"], locator);
        check!(eq; resource["resource_error"], origin);
        check!(resource.get("verdict").is_none());
        let result = journal_snapshot(&store, crate::milesplit::RESULT_SET_PHASE)?;
        check!(eq; result.values().filter(|row| row["disposition"] == "resource_limit").count(), 1);
        check!(!result.values().any(|row| row["disposition"] == "projection_applied"));
        drop(store);
        drop(fetcher);
        let moved = tempfile::tempdir()?;
        std::fs::rename(dir.path().join("store"), moved.path().join("moved-store"))?;
        std::fs::rename(dir.path().join("http"), moved.path().join("moved-http"))?;
        check!(!original_path.exists());
        check!(!dir.path().join("store").exists());
        check!(!dir.path().join("http").exists());
        let store = Store::open(moved.path().join("moved-store"))?;
        let fetcher = Fetcher::new(moved.path().join("moved-http"), None,
            std::time::Duration::ZERO, HashMap::new(), vec!["milesplit.com".into()])?.with_offline(true);
        let replay = crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &supplied).await?;
        check!(eq; (replay.rows, replay.from_cache), (0, 1));
        check!(eq; replay.disposition, crate::CollectionDisposition::Partial);
        check!(eq; replay.unfinished, report.unfinished);
        check!(eq; fetcher.stats().await.cache_hits, 1);
        check!(replay.notes.iter().any(|note| note == "supplied result locators left unread/unfinished after resource admission: 1"));
        check!(!replay.notes.iter().any(|note| note.contains(&original_path.display().to_string())));
        let current_path = retained_witness(&store, &fetcher, &reference, &body, &origin).await?;
        check!(current_path != original_path);
        check!(owned.keys().all(|key| key.starts_with("partial/resource-v2/725218/")));
        check!(eq; resource["immutable_body"], format!("archive/bodies/{}.body", crate::net::cache::content_digest(&body)));
        let (key, root) = result.iter().find(|(_, row)| row["disposition"] == "resource_limit").ok_or("stable collector root")?;
        check!(eq; key, &format!("partial/resource-v2/{}", census_domain::model::serialized_digest(root)?));
        check!(eq; root["resource_error"], origin);
        check!(replay.notes.iter().any(|note| note.contains(&current_path.display().to_string())
            && note.contains(&locator) && note.contains(&reference.url)));
        check!(eq; phases.iter().map(|phase| journal_snapshot(&store, phase)).collect::<TestResult<Vec<_>>>()?, original);
        no_canonical_effects(&store)?;
        Ok(())
    })
}
