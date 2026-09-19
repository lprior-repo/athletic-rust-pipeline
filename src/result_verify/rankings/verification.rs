use crate::{domain::identity::EvidenceDigest, runtime::{identity::fingerprint,
    protocol::{DocumentReceipt, FetchOutcome, RankingsCapture, SourceResource},
    rankings::{EventCatalog, RankingsPlan},
    rankings_collection::{collection_fingerprint, CollectionFinalSnapshot, RankingCollectionRef},
    run_protocol::SourceSnapshot}, store::ArtifactStore};
use anyhow::{bail, Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use url::Url;

pub(super) fn load<T: DeserializeOwned + Serialize>(store: &ArtifactStore, digest: &EvidenceDigest) -> Result<T> {
    let bytes = store.get_bytes(digest)?;
    let value = serde_json::from_slice(&bytes)?;
    if serde_json::to_vec(&value)? != bytes { bail!("ranking artifact differs from canonical serialization"); }
    Ok(value)
}

pub(super) fn collection(
    bound: &RankingCollectionRef, source_digest: &EvidenceDigest, store: &ArtifactStore,
) -> Result<(CollectionFinalSnapshot, RankingsPlan, Url)> {
    if store.ranking_snapshot(&bound.collection)?.as_ref() != Some(&bound.snapshot) {
        bail!("ranking reference does not match immutable seal");
    }
    let snapshot: CollectionFinalSnapshot = load(store, &bound.snapshot)?;
    let source: SourceSnapshot = load(store, source_digest)?;
    let scope = source.rankings.as_ref().context("ranking source has no scope")?;
    scope.validate()?;
    if snapshot.source_snapshot != *source_digest || snapshot.collection != bound.collection
        || collection_fingerprint(&scope.revision, source_digest)? != bound.collection
        || fingerprint(scope)? != fingerprint(&snapshot.scope)? {
        bail!("ranking final snapshot differs from its source, scope, or collection fingerprint");
    }
    let origin = super::super::source_receipts::source_origin(source_digest, store)?;
    let resource = SourceResource::Rankings {
        collection: bound.collection.clone(), list_id: scope.list_id, gender: scope.gender.clone(),
        grade: Some(scope.projection_grade), event_short: "100m".into(), page: 1,
        capture: RankingsCapture::Navigation,
    };
    let raw = outcome(&snapshot.catalog_outcome, &resource, &origin, store)?;
    let catalog = EventCatalog::from_nav(&raw, &scope.requested_families)?;
    if catalog.list_id != scope.list_id || catalog.season_id != scope.season {
        bail!("navigation scope differs from frozen collection scope");
    }
    let retained: EventCatalog = load(store, snapshot.catalog_ref.as_ref().context("missing catalog")?)?;
    if serde_json::to_value(&catalog)? != serde_json::to_value(&retained)? {
        bail!("catalog differs from retained raw navigation response");
    }
    let absent: Vec<_> = catalog.absent_families.iter().map(|family| family.family.clone()).collect();
    let plan = catalog.into_plan(bound.collection.clone(), scope.projection_grade)?;
    let retained_plan: RankingsPlan = load(store, snapshot.plan_ref.as_ref().context("missing plan")?)?;
    if serde_json::to_value(&plan)? != serde_json::to_value(retained_plan)? {
        bail!("ranking plan differs from independently expanded catalog");
    }
    if snapshot.coverage.total_requested != plan.events.len() as u64
        || snapshot.coverage.completed != snapshot.coverage.total_requested
        || snapshot.coverage.absent_families != absent
        || snapshot.event_heads.len() != plan.events.len()
        || snapshot.event_heads.iter().zip(&plan.events).any(|(head, event)| {
            head.event_short != event.short || !head.terminal || head.head_checkpoint.is_none()
                || head.page_count == 0 || head.page_count > scope.max_pages_per_event
        }) || snapshot.unique_athletes != store.ranking_collection_stats(&bound.collection)?.unique_athletes {
        bail!("sealed ranking coverage differs from catalog, event heads, or store");
    }
    Ok((snapshot, plan, origin))
}

pub(super) fn outcome(
    outcome: &FetchOutcome, resource: &SourceResource, origin: &Url, store: &ArtifactStore,
) -> Result<serde_json::Value> {
    let FetchOutcome::Retrieved { receipt, retries, previous_responses } = outcome else {
        bail!("ranking witness contains failed acquisition");
    };
    super::super::source_receipts::verify_operation(resource, retries, receipt, previous_responses, origin, store)?;
    super::super::source_receipts::verify_receipt(receipt, origin, store)?;
    capture(receipt, resource, origin)?;
    Ok(serde_json::from_slice(&store.get_bytes(&receipt.digest)?)?)
}

fn capture(receipt: &DocumentReceipt, resource: &SourceResource, origin: &Url) -> Result<()> {
    let SourceResource::Rankings { list_id, gender, grade, event_short, page, capture, .. } = resource else {
        bail!("ranking verifier received a non-ranking resource");
    };
    let evidence = receipt.rankings.as_ref().context("ranking receipt has no browser capture metadata")?;
    let url = Url::parse(&evidence.request_url)?;
    let (path, method) = match capture {
        RankingsCapture::Navigation => ("/api/v1/tfRankings/GetNavInfo", "GET"),
        RankingsCapture::Results => ("/api/v1/tfRankings/GetRankings", "POST"),
    };
    if evidence.capture != *capture || evidence.request_method != method || url.path() != path
        || url.origin() != origin.origin() || !url.username().is_empty() || url.password().is_some()
        || url.fragment().is_some() || !(200..300).contains(&receipt.http_status) {
        bail!("ranking capture kind, method, origin, route, or status differs from expected source");
    }
    if *capture == RankingsCapture::Navigation {
        if evidence.request_body.is_some() { bail!("navigation capture unexpectedly contains a request body"); }
        return Ok(());
    }
    let body: serde_json::Value = serde_json::from_str(evidence.request_body.as_deref().context("missing rankings request body")?)?;
    let grades: Vec<u8> = grade.iter().copied().collect();
    if body.get("divListId").and_then(serde_json::Value::as_u64) != Some(*list_id)
        || body.get("gender").and_then(serde_json::Value::as_str) != Some(gender.as_str())
        || body.get("eventShort").and_then(serde_json::Value::as_str) != Some(event_short.as_str())
        || body.pointer("/qParams/page").and_then(serde_json::Value::as_u64) != Some(u64::from(*page))
        || body.pointer("/qParams/grades") != Some(&serde_json::to_value(grades)?) {
        bail!("application-generated request body differs from the expected ranking scope");
    }
    Ok(())
}
