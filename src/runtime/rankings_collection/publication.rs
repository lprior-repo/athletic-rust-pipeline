mod projection;

use super::*;
use crate::{
    runtime::{
        protocol::{DocumentReceipt, FetchOutcome, RankingsCapture},
        rankings::{
            parse_page_response, EventCatalog, ExpectedPageContext, PageObservation, RankingsScope,
        },
    },
    store::{
        ArtifactStore, RankingCandidateEntry, RankingCandidateKind, RankingPageIndex,
        RankingRosterObservation, RankingSourceRow,
    },
};
use anyhow::{bail, Context};

#[derive(Serialize, Deserialize)]
pub(super) struct CatalogPublication {
    pub catalog: EvidenceDigest,
    pub plan: EvidenceDigest,
    pub events: Vec<EventProgress>,
    pub absent: Vec<String>,
}
#[derive(Serialize, Deserialize)]
pub(super) struct PagePublication {
    pub checkpoint: EvidenceDigest,
    pub terminal: bool,
}

fn put<T: Serialize>(store: &ArtifactStore, value: &T) -> anyhow::Result<EvidenceDigest> {
    Ok(store.put_bytes(&serde_json::to_vec(value)?)?)
}
fn receipt(outcome: &FetchOutcome) -> anyhow::Result<&DocumentReceipt> {
    let FetchOutcome::Retrieved { receipt, .. } = outcome else {
        bail!("ranking acquisition failed");
    };
    if !(200..300).contains(&receipt.http_status) {
        bail!("ranking response is not successful");
    }
    Ok(receipt)
}
fn raw(store: &ArtifactStore, receipt: &DocumentReceipt) -> anyhow::Result<serde_json::Value> {
    let bytes = store.get_bytes(&receipt.digest)?;
    if u64::try_from(bytes.len())? != receipt.bytes {
        bail!("ranking receipt length mismatch");
    }
    Ok(serde_json::from_slice(&bytes)?)
}

pub(super) fn catalog(
    store: &ArtifactStore,
    scope: &RankingsScope,
    collection: EvidenceDigest,
    outcome: &FetchOutcome,
) -> anyhow::Result<CatalogPublication> {
    let receipt = receipt(outcome)?;
    let capture = receipt
        .rankings
        .as_ref()
        .context("catalog capture metadata missing")?;
    if capture.capture != RankingsCapture::Navigation {
        bail!("wrong catalog capture kind");
    }
    let mut catalog = EventCatalog::from_nav(
        &raw(store, receipt)?,
        &scope.requested_families,
        scope.season_kind,
        scope.list_id,
    )?;
    if catalog.list_id != scope.list_id || catalog.season_id != scope.source_season_id() {
        bail!("catalog list or season differs from requested scope");
    }
    let catalog_digest = put(store, &catalog)?;
    let absent = std::mem::take(&mut catalog.absent_families)
        .into_iter()
        .map(|family| family.family)
        .collect();
    let plan = catalog.into_plan(collection, scope.projection_grade, &scope.gender)?;
    let plan_digest = put(store, &plan)?;
    let events = plan
        .events
        .into_iter()
        .map(|event| EventProgress {
            event_short: event.short,
            event_id: event.event_id,
            is_relay: event.is_relay,
            next_page: 1,
            head_checkpoint: None,
            page_count: 0,
            terminal: false,
        })
        .collect();
    Ok(CatalogPublication {
        catalog: catalog_digest,
        plan: plan_digest,
        events,
        absent,
    })
}

pub(super) fn page(
    store: &ArtifactStore,
    expected: &ExpectedPageContext<'_>,
    revision: String,
    event: &EventProgress,
    collection: EvidenceDigest,
    outcome: FetchOutcome,
) -> anyhow::Result<PagePublication> {
    let (receipt, terminal) = validated_capture(&outcome, event)?;
    let observation = parse_page_response(&raw(store, receipt)?, expected)?;
    let min_count = observation.min_count;
    let index_collection = collection.clone();
    let checkpoint = RankingsPageCheckpoint {
        revision,
        collection,
        event_short: event.event_short.clone(),
        page: event.next_page,
        previous_checkpoint: event.head_checkpoint.clone(),
        outcome,
        observation_digest: put(store, &observation)?,
    };
    let digest = put(store, &checkpoint)?;
    let index = page_index(
        observation,
        index_collection,
        &event.event_short,
        event.next_page,
        digest.clone(),
    )?;
    store.drop_rankings_page(&index.collection, &event.event_short, event.next_page)?;
    store.put_rankings_page(&index)?;
    if terminal {
        warn_terminal_shortfall(store, &index.collection, &event.event_short, min_count)?;
    }
    Ok(PagePublication {
        checkpoint: digest,
        terminal,
    })
}

/// Validate the acquired document's ranking capture metadata against the event
/// cursor, returning its receipt and whether the page terminates the event.
fn validated_capture<'a>(
    outcome: &'a FetchOutcome,
    event: &EventProgress,
) -> anyhow::Result<(&'a DocumentReceipt, bool)> {
    let receipt = receipt(outcome)?;
    let capture = receipt
        .rankings
        .as_ref()
        .context("ranking capture metadata missing")?;
    if capture.capture != RankingsCapture::Results {
        bail!("wrong results capture kind");
    }
    if capture
        .next_page
        .is_some_and(|next| event.next_page.checked_add(1) != Some(next))
    {
        bail!("pagination does not advance to the next consecutive page");
    }
    Ok((receipt, capture.next_page.is_none()))
}

/// Warn when a terminal page is short of the source's declared minCount bound.
///
/// The source's minCount is not always reachable. The outdoor boys grade
/// 11 200m listing declares 654 rows while its own pagination widget
/// renders every page after the first as disabled (measured 2026-09-21
/// through the source UI in an independent browser with a trusted click)
/// and its API answers a page-2 request with a byte-identical page-1
/// body. Sealing with the rows the source actually served keeps the
/// element honest: the terminal observation retains the declared
/// minCount, so the shortfall stays visible in the retained evidence
/// instead of wedging the collection on an unreachable bound.
fn warn_terminal_shortfall(
    store: &ArtifactStore,
    collection: &EvidenceDigest,
    event_short: &str,
    min_count: u64,
) -> anyhow::Result<()> {
    let stats = store.ranking_event_stats(collection, event_short)?;
    if stats.row_positions < min_count || stats.max_row_position < min_count {
        tracing::warn!(
            event = %event_short,
            min_count,
            row_positions = stats.row_positions,
            max_row_position = stats.max_row_position,
            pages = stats.pages,
            "terminal ranking page is short of the source minCount lower bound"
        );
    }
    Ok(())
}

fn page_index(
    observation: PageObservation,
    collection: EvidenceDigest,
    event: &str,
    page: u32,
    checkpoint: EvidenceDigest,
) -> anyhow::Result<RankingPageIndex> {
    let (candidates, rows, rosters) = projection::index_parts(observation)?;
    Ok(RankingPageIndex {
        collection,
        event_short: event.to_owned(),
        page,
        checkpoint,
        candidates,
        rows,
        rosters,
    })
}

pub(super) struct SealInput {
    pub scope: RankingsScope,
    pub source_snapshot: EvidenceDigest,
    pub catalog_outcome: FetchOutcome,
    pub catalog_ref: Option<EvidenceDigest>,
    pub plan_ref: Option<EvidenceDigest>,
    pub events: Vec<EventProgress>,
    pub absent_families: Vec<String>,
}

pub(super) fn seal(
    store: &ArtifactStore,
    input: SealInput,
    collection: EvidenceDigest,
) -> anyhow::Result<EvidenceDigest> {
    let SealInput {
        scope,
        source_snapshot,
        catalog_outcome,
        catalog_ref,
        plan_ref,
        events,
        absent_families,
    } = input;
    if events.is_empty()
        || events
            .iter()
            .any(|event| !event.terminal || event.head_checkpoint.is_none())
    {
        bail!("cannot seal an incomplete ranking collection");
    }
    let unique_athletes = store.ranking_collection_stats(&collection)?.unique_athletes;
    let total_requested = u64::try_from(events.len())?;
    let snapshot = CollectionFinalSnapshot {
        collection,
        source_snapshot,
        scope,
        catalog_outcome,
        catalog_ref,
        plan_ref,
        event_heads: events
            .into_iter()
            .map(|event| EventHeadRef {
                event_short: event.event_short,
                head_checkpoint: event.head_checkpoint,
                page_count: event.page_count,
                terminal: event.terminal,
            })
            .collect(),
        coverage: CoverageSummary {
            total_requested,
            completed: total_requested,
            absent_families,
        },
        unique_athletes,
    };
    let digest = put(store, &snapshot)?;
    store.seal_rankings(&snapshot.collection, &digest)?;
    Ok(digest)
}
