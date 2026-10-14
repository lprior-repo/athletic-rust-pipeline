use super::*;
use crate::milesplit::OwnedMeetVerdict;
use base64::{engine::general_purpose::STANDARD, Engine};

async fn historical_resource(body: &[u8], old_detail: String, locator: &str) -> TestResult {
    let (dir, store, fetcher, reference) = setup()?;
    seed_owned(&fetcher, &reference, body)?;
    seed_metadata(&fetcher, &reference)?;
    let ctx = context(&store, &fetcher)?;
    let mut capture =
        crate::milesplit::fetch::fetch_owned_capture(&fetcher, &reference, &ctx.fetch_options())
            .await?;
    capture.from_cache = false;
    let prefix = format!("725218/{}", capture.content_digest);
    let content_manifest = format!("{prefix}/manifest");
    let acquisition = crate::milesplit::owned::acquisition_manifest_key(725218, &capture)?;
    let old_key = format!(
        "partial/{}",
        acquisition
            .strip_suffix("/manifest")
            .ok_or("manifest suffix")?
    );
    let old_verdict = OwnedMeetVerdict::Malformed { detail: old_detail };
    let old = json!({"capture": &capture, "verdict": old_verdict});
    let original_bytes = serde_json::to_vec(&old)?;
    let mut batch = store.write_batch();
    for (index, chunk) in capture.body.chunks(32 * 1024).enumerate() {
        batch.journal_done(crate::milesplit::OWNED_CAPTURE_PHASE, &format!("{prefix}/{index}"),
            &json!({"capture": &capture, "chunk_index": index, "raw_base64": STANDARD.encode(chunk)}))?;
    }
    let chunks = capture.body.chunks(32 * 1024).len();
    batch.journal_done(crate::milesplit::OWNED_CAPTURE_PHASE, &content_manifest,
        &json!({"capture": &capture, "chunk_bytes": 32 * 1024, "encoding": "base64", "chunks": chunks}))?;
    batch.journal_done(crate::milesplit::OWNED_CAPTURE_PHASE, &acquisition,
        &json!({"capture": &capture, "content_manifest": content_manifest, "chunk_key_prefix": prefix,
            "chunk_bytes": 32 * 1024, "encoding": "base64", "chunks": chunks}))?;
    batch.journal_done(crate::milesplit::OWNED_MEET_PHASE, &old_key, &old)?;
    batch.journal_done("milesplit_owned_meet_v3", "partial/legacy", &old)?;
    batch.commit()?;
    let archived = journal_snapshot(&store, crate::milesplit::OWNED_CAPTURE_PHASE)?;
    let legacy = journal_snapshot(&store, "milesplit_owned_meet_v3")?;
    let second = ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266815/raw")
        .ok_or("later locator")?;
    let mut supplied = options(&reference);
    supplied.urls.extend(options(&second).urls);
    let report =
        crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &supplied).await?;
    check!(eq; (report.rows, report.from_cache), (0, 1));
    check!(eq; report.disposition, crate::CollectionDisposition::Partial);
    check!(eq; report.unfinished, supplied.urls.iter().map(|request| request.url.clone()).collect::<Vec<_>>());
    no_canonical_effects(&store)?;
    check!(eq; crate::milesplit::OWNED_MEET_PHASE, "milesplit_owned_meet_v4");
    check!(eq; crate::milesplit::OWNED_CAPTURE_PHASE, "milesplit_owned_capture_v1");
    let preserved = store
        .journal_payload(crate::milesplit::OWNED_MEET_PHASE, &old_key)?
        .ok_or("old malformed receipt")?;
    check!(eq; serde_json::to_vec(&preserved)?, original_bytes);
    check!(eq; journal_snapshot(&store, crate::milesplit::OWNED_CAPTURE_PHASE)?, archived);
    check!(eq; journal_snapshot(&store, "milesplit_owned_meet_v3")?, legacy);
    let current = journal_snapshot(&store, crate::milesplit::OWNED_MEET_PHASE)?;
    let resources = current
        .iter()
        .filter(|(key, _)| key.starts_with("partial/resource-v2/"))
        .collect::<Vec<_>>();
    check!(eq; resources.len(), 1);
    let resource = &resources.first().ok_or("new resource receipt")?.1;
    check!(eq; resource["unfinished_locator"], locator);
    check!(eq; resource["disposition"], "resource_limit");
    check!(resource.get("verdict").is_none());
    check!(eq; std::fs::read(fetcher.cache_dir().join(resource["immutable_body"].as_str().ok_or("original body locator")?))?, body);
    let gaps = journal_snapshot(&store, crate::milesplit::RESULT_SET_PHASE)?;
    check!(!gaps
        .values()
        .any(|payload| payload["disposition"] == "projection_applied"));
    drop(store);
    let store = Store::open(dir.path().join("store"))?;
    let replay =
        crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &supplied).await?;
    check!(eq; (replay.rows, replay.unfinished), (0, report.unfinished));
    no_canonical_effects(&store)?;
    check!(eq; journal_snapshot(&store, crate::milesplit::OWNED_MEET_PHASE)?, current);
    check!(eq; journal_snapshot(&store, crate::milesplit::OWNED_CAPTURE_PHASE)?, archived);
    check!(eq; journal_snapshot(&store, crate::milesplit::RESULT_SET_PHASE)?, gaps);
    check!(eq; journal_snapshot(&store, "milesplit_owned_meet_v3")?, legacy);
    Ok(())
}

#[test]
fn historical_resource_misclassified_as_malformed_never_suppresses_the_real_collector_stop(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut source = document(725218, 1, 1)?;
            source["providerExtension"] = json!(vec![0; 20_001]);
            let body = serde_json::to_vec(&source)?;
            let offset = body.len().checked_sub(3).ok_or("provider offset")?;
            historical_resource(
                &body,
                "owned array values reservation of 20001 exceeds limit 20000".into(),
                &format!("byte[{offset}]"),
            )
            .await
        })
}

#[test]
fn historical_large_malformed_envelope_bytes_survive_the_new_diagnostic_resource_bound(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let text = "é".repeat(2_000);
            let body = serde_json::to_vec(&json!({"_meta": {"status_code": text}, "data": []}))?;
            let detail = format!(
                "provider envelope status is {}",
                serde_json::to_string(&text)?
            );
            historical_resource(&body, detail, "/_meta/status_code").await
        })
}
