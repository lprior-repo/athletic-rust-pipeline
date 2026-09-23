use super::{load, outcome};
use crate::{
    domain::identity::EvidenceDigest,
    result_verify::source_receipts,
    runtime::{
        protocol::SourceResource,
        rankings::{EventCatalog, RankingsPlan, RankingsScope},
        rankings_collection::{CollectionFinalSnapshot, RankingCollectionRef},
    },
    store::ArtifactStore,
};
use anyhow::{bail, Context, Result};
use athleticnet_browser::protocol::{RankingPageObservation, RankingsCapture};
use url::Url;

pub(super) fn verify_navigation(
    bound: &RankingCollectionRef,
    source_digest: &EvidenceDigest,
    scope: &RankingsScope,
    snapshot: &CollectionFinalSnapshot,
    store: &ArtifactStore,
) -> Result<(EventCatalog, Url)> {
    let origin = source_receipts::source_origin(source_digest, store)?;
    let resource = SourceResource::Rankings {
        collection: bound.collection.clone(),
        list_id: scope.list_id,
        gender: scope.gender.clone(),
        grade: Some(scope.projection_grade),
        event_short: "100m".into(),
        page: 1,
        capture: RankingsCapture::Navigation,
    };
    let raw = outcome(&snapshot.catalog_outcome, &resource, &origin, store)?;
    let catalog = EventCatalog::from_nav(
        &raw,
        &scope.requested_families,
        scope.season_kind,
        scope.list_id,
    )?;
    if catalog.list_id != scope.list_id || catalog.season_id != scope.source_season_id() {
        bail!("navigation scope differs from frozen collection scope");
    }
    let retained: EventCatalog = load(
        store,
        snapshot.catalog_ref.as_ref().context("missing catalog")?,
    )?;
    if serde_json::to_value(&catalog)? != serde_json::to_value(&retained)? {
        bail!("catalog differs from retained raw navigation response");
    }
    Ok((catalog, origin))
}

pub(super) fn verify_catalog(
    bound: &RankingCollectionRef,
    scope: &RankingsScope,
    snapshot: &CollectionFinalSnapshot,
    catalog: EventCatalog,
    store: &ArtifactStore,
) -> Result<(RankingsPlan, Vec<String>)> {
    let absent: Vec<_> = catalog
        .absent_families
        .iter()
        .map(|family| family.family.clone())
        .collect();
    let plan = catalog.into_plan(
        bound.collection.clone(),
        scope.projection_grade,
        &scope.gender,
    )?;
    let retained_plan: RankingsPlan =
        load(store, snapshot.plan_ref.as_ref().context("missing plan")?)?;
    if serde_json::to_value(&plan)? != serde_json::to_value(retained_plan)? {
        bail!("ranking plan differs from independently expanded catalog");
    }
    Ok((plan, absent))
}

pub(super) fn verify_sealed_coverage(
    bound: &RankingCollectionRef,
    scope: &RankingsScope,
    snapshot: &CollectionFinalSnapshot,
    plan: &RankingsPlan,
    absent: &[String],
    store: &ArtifactStore,
) -> Result<()> {
    if snapshot.coverage.total_requested != u64::try_from(plan.events.len())?
        || snapshot.coverage.completed != snapshot.coverage.total_requested
        || snapshot.coverage.absent_families != absent
        || snapshot.event_heads.len() != plan.events.len()
        || snapshot
            .event_heads
            .iter()
            .zip(&plan.events)
            .any(|(head, event)| {
                head.event_short != event.short
                    || !head.terminal
                    || head.head_checkpoint.is_none()
                    || head.page_count == 0
                    || head.page_count > scope.max_pages_per_event
            })
        || snapshot.unique_athletes
            != store
                .ranking_collection_stats(&bound.collection)?
                .unique_athletes
    {
        bail!("sealed ranking coverage differs from catalog, event heads, or store");
    }
    Ok(())
}

pub(super) fn verify_capture_body(
    evidence: &RankingPageObservation,
    list_id: u64,
    gender: &str,
    grade: Option<u8>,
    event_short: &str,
    page: u32,
) -> Result<()> {
    let body: serde_json::Value = serde_json::from_str(
        evidence
            .request_body
            .as_deref()
            .context("missing rankings request body")?,
    )?;
    let grades: Vec<u8> = grade.iter().copied().collect();
    if body.get("divListId").and_then(serde_json::Value::as_u64) != Some(list_id)
        || body.get("gender").and_then(serde_json::Value::as_str) != Some(gender)
        || body.get("eventShort").and_then(serde_json::Value::as_str) != Some(event_short)
        || body
            .pointer("/qParams/page")
            .and_then(serde_json::Value::as_u64)
            != Some(u64::from(page))
        || body.pointer("/qParams/grades") != Some(&serde_json::to_value(grades)?)
    {
        bail!("application-generated request body differs from the expected ranking scope");
    }
    Ok(())
}
