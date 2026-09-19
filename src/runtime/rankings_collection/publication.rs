use super::*;
use crate::{runtime::{protocol::{DocumentReceipt, FetchOutcome, RankingsCapture},
    rankings::{EventCatalog, ExpectedPageContext, PageObservation, RequestedFamily, parse_page_response}},
    store::{ArtifactStore, RankingCandidateEntry, RankingCandidateKind, RankingPageIndex,
        RankingRosterObservation, RankingSourceRow}};
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
    let FetchOutcome::Retrieved { receipt, .. } = outcome else { bail!("ranking acquisition failed"); };
    if !(200..300).contains(&receipt.http_status) { bail!("ranking response is not successful"); }
    Ok(receipt)
}
fn raw(store: &ArtifactStore, receipt: &DocumentReceipt) -> anyhow::Result<serde_json::Value> {
    let bytes = store.get_bytes(&receipt.digest)?;
    if bytes.len() as u64 != receipt.bytes { bail!("ranking receipt length mismatch"); }
    Ok(serde_json::from_slice(&bytes)?)
}

pub(super) fn catalog(
    store: &ArtifactStore, requested_families: &[RequestedFamily], list_id: u64, season: u64,
    projection_grade: u8, collection: EvidenceDigest, outcome: &FetchOutcome,
) -> anyhow::Result<CatalogPublication> {
    let receipt = receipt(outcome)?;
    let capture = receipt.rankings.as_ref().context("catalog capture metadata missing")?;
    if capture.capture != RankingsCapture::Navigation { bail!("wrong catalog capture kind"); }
    let mut catalog = EventCatalog::from_nav(&raw(store, receipt)?, requested_families)?;
    if catalog.list_id != list_id || catalog.season_id != season {
        bail!("catalog list or season differs from requested scope");
    }
    let catalog_digest = put(store, &catalog)?;
    let absent = std::mem::take(&mut catalog.absent_families)
        .into_iter().map(|family| family.family).collect();
    let plan = catalog.into_plan(collection, projection_grade)?;
    let plan_digest = put(store, &plan)?;
    let events = plan.events.into_iter().map(|event| EventProgress {
        event_short: event.short, event_id: event.event_id, is_relay: event.is_relay,
        next_page: 1, head_checkpoint: None, page_count: 0, terminal: false,
    }).collect();
    Ok(CatalogPublication { catalog: catalog_digest, plan: plan_digest, events, absent })
}

pub(super) fn page(
    store: &ArtifactStore, list_id: u64, season: u64, gender: &str, projection_grade: u8,
    revision: String, event: &EventProgress, collection: EvidenceDigest, outcome: FetchOutcome,
) -> anyhow::Result<PagePublication> {
    let receipt = receipt(&outcome)?;
    let capture = receipt.rankings.as_ref().context("ranking capture metadata missing")?;
    if capture.capture != RankingsCapture::Results { bail!("wrong results capture kind"); }
    if capture.next_page.is_some_and(|next| event.next_page.checked_add(1) != Some(next)) {
        bail!("pagination does not advance to the next consecutive page");
    }
    let terminal = capture.next_page.is_none();
    let observation = parse_page_response(&raw(store, receipt)?, &ExpectedPageContext {
        division_id: list_id, season_id: season, gender,
        event_short: &event.event_short, event_id: Some(event.event_id), is_relay: event.is_relay,
        requested_grade: if event.is_relay { None } else { Some(projection_grade) }, page: event.next_page,
    })?;
    let min_count = observation.min_count;
    let index_collection = collection.clone();
    let checkpoint = RankingsPageCheckpoint {
        revision, collection, event_short: event.event_short.clone(),
        page: event.next_page, previous_checkpoint: event.head_checkpoint.clone(), outcome,
        observation_digest: put(store, &observation)?,
    };
    let digest = put(store, &checkpoint)?;
    let index = page_index(observation, index_collection, &event.event_short, event.next_page, digest.clone())?;
    store.put_rankings_page(&index)?;
    let stats = store.ranking_event_stats(&index.collection, &event.event_short)?;
    if terminal && (stats.row_positions < min_count || stats.max_row_position < min_count) {
        bail!("terminal ranking page does not cover the source minCount lower bound");
    }
    Ok(PagePublication { checkpoint: digest, terminal })
}

fn page_index(
    observation: PageObservation, collection: EvidenceDigest, event: &str, page: u32, checkpoint: EvidenceDigest,
) -> anyhow::Result<RankingPageIndex> {
    let PageObservation {
        grade_11_candidates_list, verified_relay_members, source_rows, ..
    } = observation;
    let individual = grade_11_candidates_list.into_iter().enumerate().map(|(index, candidate)| {
        Ok(RankingCandidateEntry { name: candidate.name, athlete_id: candidate.athlete_id,
            kind: RankingCandidateKind::Individual, record_index: u32::try_from(index)?, result_id: candidate.id_result })
    });
    let relay = verified_relay_members.into_iter().enumerate().map(|(index, member)| {
        Ok(RankingCandidateEntry { name: member.name, athlete_id: member.athlete_id,
            kind: RankingCandidateKind::RelayMember, record_index: u32::try_from(index)?, result_id: member.id_result })
    });
    Ok(RankingPageIndex {
        collection, event_short: event.to_owned(), page, checkpoint,
        candidates: individual.chain(relay).collect::<anyhow::Result<Vec<_>>>()?,
        rows: source_rows.iter().map(|row| RankingSourceRow {
            result_id: row.result_id, row_number: row.row_number,
        }).collect(),
        rosters: source_rows.iter().filter_map(|row| row.roster_present.map(|present| {
            RankingRosterObservation { result_id: row.result_id, present }
        })).collect(),
    })
}

pub(super) fn seal(
    store: &ArtifactStore, scope: RankingsScope, source_snapshot: EvidenceDigest,
    catalog_outcome: FetchOutcome, catalog_ref: Option<EvidenceDigest>,
    plan_ref: Option<EvidenceDigest>, events: Vec<EventProgress>,
    absent_families: Vec<String>, collection: EvidenceDigest,
) -> anyhow::Result<EvidenceDigest> {
    if events.is_empty() || events.iter().any(|event| !event.terminal || event.head_checkpoint.is_none()) {
        bail!("cannot seal an incomplete ranking collection");
    }
    let unique_athletes = store.ranking_collection_stats(&collection)?.unique_athletes;
    let total_requested = u64::try_from(events.len())?;
    let snapshot = CollectionFinalSnapshot {
        collection, source_snapshot, scope, catalog_outcome,
        catalog_ref, plan_ref,
        event_heads: events.into_iter().map(|event| EventHeadRef {
            event_short: event.event_short, head_checkpoint: event.head_checkpoint,
            page_count: event.page_count, terminal: event.terminal,
        }).collect(),
        coverage: CoverageSummary { total_requested,
            completed: total_requested, absent_families },
        unique_athletes,
    };
    let digest = put(store, &snapshot)?;
    store.seal_rankings(&snapshot.collection, &digest)?;
    Ok(digest)
}
