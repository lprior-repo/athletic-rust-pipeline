use super::*;
use crate::milesplit::{OwnedMeetParseError, OwnedResourceReceipt};

type LegacyPayloads = Vec<(&'static str, String, Vec<u8>)>;

async fn seed_legacy_resources(
    ctx: &AdapterContext<'_>,
    reference: &ResultSetRef,
    body: &[u8],
) -> TestResult<LegacyPayloads> {
    let mut captured =
        crate::milesplit::fetch::fetch_owned_capture(ctx, reference, &ctx.fetch_options())
            .await?
            .capture;
    captured.from_cache = false;
    let failure = match crate::milesplit::parse_owned_meet(body, 725218) {
        Err(OwnedMeetParseError::Resource(failure)) => failure,
        other => return Err(format!("legacy resource fixture: {other:?}").into()),
    };
    let origin = failure.to_string();
    let manifest = crate::milesplit::owned::acquisition_manifest_key(725218, &captured)?;
    let archived = ctx
        .fetcher
        .cache_dir()
        .join("archive")
        .join("bodies")
        .join(format!("{}.body", captured.content_digest));
    check!(archived.is_absolute());
    check!(eq; std::fs::read(&archived)?, body);
    let owned_key = format!(
        "partial/resource/725218/{}",
        census_domain::model::serialized_digest(&(manifest, &origin))?
    );
    let owned = json!({
        "capture": &captured, "disposition": "resource_limit", "unfinished": true,
        "capture_available": true, "immutable_body": &archived,
        "unfinished_locator": failure.locator().to_string(), "resource_error": origin,
    });
    let provenance = crate::milesplit::results::run::capture::provenance(&captured);
    let retained = failure.retain(captured, archived, OwnedResourceReceipt::Retained);
    let result = json!({
        "source_url": reference.url, "owned_capture": provenance, "raw_metadata_capture": null,
        "performance_as_of": ctx.performance_as_of,
        "disposition": "resource_limit", "unfinished": true, "resource_error": retained.to_string(),
    });
    let result_key = format!(
        "partial/resource/{}",
        census_domain::model::serialized_digest(&result)?
    );
    let legacy = [
        (crate::milesplit::OWNED_MEET_PHASE, owned_key, owned),
        (crate::milesplit::RESULT_SET_PHASE, result_key, result),
    ];
    let mut batch = ctx.store.write_batch();
    for (phase, key, payload) in &legacy {
        batch.journal_done(phase, key, payload)?;
    }
    batch.commit()?;
    legacy
        .into_iter()
        .map(|(phase, key, payload)| Ok((phase, key, serde_json::to_vec(&payload)?)))
        .collect()
}

fn assert_legacy_resources(store: &Store, legacy: &LegacyPayloads) -> TestResult {
    for (phase, key, bytes) in legacy {
        let payload = store
            .journal_payload(phase, key)?
            .ok_or("original v1 receipt")?;
        check!(eq; serde_json::to_vec(&payload)?, *bytes);
        check!(eq; store.journal_keys(phase)?.iter().filter(|key| key.starts_with("partial/resource/")).count(), 1);
        let current = journal_snapshot(store, phase)?;
        let v2 = current
            .iter()
            .filter(|(key, _)| key.starts_with("partial/resource-v2/"))
            .collect::<Vec<_>>();
        check!(eq; v2.len(), 1);
        let (_, resource) = v2.first().ok_or("one v2 resource root")?;
        check!(eq; resource["disposition"], "resource_limit");
        check!(eq; resource["unfinished"], true);
        check!(resource.get("verdict").is_none());
        check!(!current
            .values()
            .any(|payload| payload["disposition"] == "projection_applied"));
    }
    no_canonical_effects(store)
}

#[test]
fn old_absolute_resource_receipts_survive_v2_collection_and_physical_relocation() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (dir, store, fetcher, reference) = setup()?;
        let mut source = document(725218, 1, 1)?;
        source["providerExtension"] = json!(vec![0; 20_001]);
        let body = serde_json::to_vec(&source)?;
        seed_owned(&fetcher, &reference, &body)?;
        seed_metadata(&fetcher, &reference)?;
        let legacy = seed_legacy_resources(&context(&store, &fetcher)?, &reference, &body).await?;
        let second = ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266815/raw")
            .ok_or("later locator")?;
        let mut supplied = options(&reference);
        supplied.urls.extend(options(&second).urls);
        let report = crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &supplied).await?;
        check!(eq; (report.rows, report.from_cache), (0, 1));
        check!(eq; report.disposition, crate::CollectionDisposition::Partial);
        check!(eq; report.unfinished, supplied.urls.iter().map(|request| request.url.clone()).collect::<Vec<_>>());
        assert_legacy_resources(&store, &legacy)?;
        let phases = [crate::milesplit::OWNED_MEET_PHASE, crate::milesplit::OWNED_CAPTURE_PHASE, crate::milesplit::RESULT_SET_PHASE];
        let original = phases.iter().map(|phase| journal_snapshot(&store, phase)).collect::<TestResult<Vec<_>>>()?;
        let old_body = fetcher.cache_dir().join("archive").join("bodies")
            .join(format!("{}.body", crate::net::cache::content_digest(&body)));
        drop(store);
        drop(fetcher);
        let moved = tempfile::tempdir()?;
        std::fs::rename(dir.path().join("store"), moved.path().join("store"))?;
        std::fs::rename(dir.path().join("http"), moved.path().join("http"))?;
        check!(!old_body.exists());
        let store = Store::open(moved.path().join("store"))?;
        let fetcher = Fetcher::new(moved.path().join("http"), None,
            std::time::Duration::ZERO, HashMap::new(), vec!["milesplit.com".into()])?.with_offline(true);
        let replay = crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &supplied).await?;
        check!(eq; (replay.rows, replay.from_cache), (0, 1));
        check!(eq; replay.disposition, crate::CollectionDisposition::Partial);
        check!(eq; replay.unfinished, report.unfinished);
        check!(eq; fetcher.stats().await.cache_hits, 1);
        check!(replay.notes.iter().any(|note| note == "supplied result locators left unread/unfinished after resource admission: 1"));
        check!(!replay.notes.iter().any(|note| note.contains(&old_body.display().to_string())));
        assert_legacy_resources(&store, &legacy)?;
        check!(eq; phases.iter().map(|phase| journal_snapshot(&store, phase)).collect::<TestResult<Vec<_>>>()?, original);
        let current = journal_snapshot(&store, crate::milesplit::OWNED_MEET_PHASE)?;
        let resource = current.iter().find(|(key, _)| key.starts_with("partial/resource-v2/")).ok_or("v2 receipt")?.1;
        let path = fetcher.cache_dir().join(resource["immutable_body"].as_str().ok_or("stable body locator")?);
        check!(eq; std::fs::read(&path)?, body);
        check!(eq; crate::net::cache::content_digest(&std::fs::read(&path)?), resource["capture"]["content_digest"].as_str().ok_or("retained digest")?);
        check!(replay.notes.iter().any(|note| note.contains(&path.display().to_string())));
        Ok(())
    })
}
