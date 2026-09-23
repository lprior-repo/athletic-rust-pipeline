use super::*;
use crate::{
    domain::identity::{EvidenceDigest, SourceRowKey, WorkbookDigest},
    runtime::{
        acquisition::ACQUISITION_REVISION,
        protocol::{DocumentReceipt, FetchOutcome},
        rankings::{
            parse_page_response, EventCatalog, ExpectedPageContext, PageObservation, RankingsScope,
            SeasonKind,
        },
        rankings_collection::{
            collection_fingerprint, CollectionFinalSnapshot, CoverageSummary, EventHeadRef,
            RankingCollectionRef, RankingsPageCheckpoint,
        },
        row_protocol::{DiscoverySummary, RankingDiscoveryEvidence, RowJob},
        run_protocol::SourceSnapshot,
    },
    store::{
        ArtifactStore, RankingCandidateEntry, RankingCandidateKind, RankingPageIndex,
        RankingRosterObservation, RankingSourceRow,
    },
};
use anyhow::Context;
use athleticnet_browser::protocol::{RankingPageObservation, RankingsCapture};
use athleticnet_browser::request::{rankings_spec, RankingsAction, RequestSpec};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use tempfile::tempdir;
use url::Url;

/// Frozen synthetic source origin. The rankings verifier accepts a captured
/// fixture origin, so the fixture does not need the live host.
const ORIGIN: &str = "http://127.0.0.1:18081/";
/// Trimmed GetNavInfo capture: level nav that keys both seasonal lists
/// (`2026` outdoor 168416, `12026` indoor 173005) and observes the single
/// `100m` event, so the derived plan holds exactly one event.
const NAV: &str = include_str!("../../../tests/fixtures/rankings/indoor-nav.json");
/// Captured indoor girls `100m` page one, three rows, two Grade 11
/// candidates. The `AthleteID` 1001 row carries the retained source name so
/// the ranking lookup and the retained row agree.
const PAGE: &str = include_str!("../../../tests/fixtures/rankings/indoor-girls-100m-p1.json");
/// The retained accepted row whose source identity the frozen collection is
/// matched against.
const RETAINED_ROW: &[u8] =
    include_bytes!("../../../fuzz/fixtures/retained_results_jsonl/seed-valid-accepted.json");
const MAX_PAGES_PER_EVENT: u32 = 50;
/// The retained page is an indoor girls page. The sealed scope may declare
/// anything, which is how the rejection cases exercise the scope binding:
/// the verifier re-derives the page context from the seal rather than
/// trusting the retained observation.
const PAGE_KIND: SeasonKind = SeasonKind::Indoor;
const PAGE_GENDER: &str = "f";

/// A frozen rankings collection as publication writes it: navigation
/// catalog, one terminal page, the index derived from that page, the sealed
/// snapshot, and the row-level discovery that references the seal.
struct Frozen {
    store: ArtifactStore,
    source: SourceRecord,
    discovery: DiscoverySummary,
    records: usize,
    _directory: tempfile::TempDir,
}

fn put<T: Serialize>(store: &ArtifactStore, value: &T) -> anyhow::Result<EvidenceDigest> {
    Ok(store.put_bytes(&serde_json::to_vec(value)?)?)
}

/// Deterministic operation identity for one captured source operation.
fn operation(digit: char) -> anyhow::Result<EvidenceDigest> {
    Ok(EvidenceDigest::parse(&digit.to_string().repeat(64))?)
}

fn captured(
    store: &ArtifactStore,
    spec: &RequestSpec,
    receipt: DocumentReceipt,
    operation: &EvidenceDigest,
) -> anyhow::Result<EvidenceDigest> {
    let status = receipt.http_status;
    let attempt = json!({
        "request": serde_json::to_value(spec)?,
        "receipt": receipt,
        "code": Value::Null,
        "status": status,
        "message": "",
        "retryable": false,
        "retry_after_ms": 0
    });
    Ok(store.record_attempt(operation, &attempt)?)
}

fn outcome(
    receipt: DocumentReceipt,
    operation: &EvidenceDigest,
    attempt: EvidenceDigest,
) -> anyhow::Result<FetchOutcome> {
    Ok(serde_json::from_value(json!({
        "outcome": "retrieved",
        "receipt": receipt,
        "retries": {
            "ownership": "workflow_controlled",
            "operation": operation,
            "maximum_retries": 3,
            "observed_attempts": 1,
            "attempts": [attempt]
        },
        "previous_responses": []
    }))?)
}

/// Index derivation mirrored from publication: individual records keep
/// their observation order, relay members follow.
fn page_index(
    observation: &PageObservation,
    collection: &EvidenceDigest,
    event_short: &str,
    page: u32,
    checkpoint: &EvidenceDigest,
) -> anyhow::Result<RankingPageIndex> {
    let individual =
        observation
            .grade_11_candidates_list
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                Ok(RankingCandidateEntry {
                    name: candidate.name.clone(),
                    athlete_id: candidate.athlete_id,
                    kind: RankingCandidateKind::Individual,
                    record_index: u32::try_from(index)?,
                    result_id: candidate.id_result,
                })
            });
    let relay = observation
        .verified_relay_members
        .iter()
        .enumerate()
        .map(|(index, member)| {
            Ok(RankingCandidateEntry {
                name: member.name.clone(),
                athlete_id: member.athlete_id,
                kind: RankingCandidateKind::RelayMember,
                record_index: u32::try_from(index)?,
                result_id: member.id_result,
            })
        });
    Ok(RankingPageIndex {
        collection: collection.clone(),
        event_short: event_short.to_owned(),
        page,
        checkpoint: checkpoint.clone(),
        candidates: individual
            .chain(relay)
            .collect::<anyhow::Result<Vec<_>>>()?,
        rows: observation
            .source_rows
            .iter()
            .map(|row| RankingSourceRow {
                result_id: row.result_id,
                row_number: row.row_number,
            })
            .collect(),
        rosters: observation
            .source_rows
            .iter()
            .filter_map(|row| {
                row.roster_present.map(|present| RankingRosterObservation {
                    result_id: row.result_id,
                    present,
                })
            })
            .collect(),
    })
}

fn frozen(kind: SeasonKind, gender: &str) -> anyhow::Result<Frozen> {
    let directory = tempdir()?;
    let store = ArtifactStore::open(&directory.path().join("artifacts"))?;
    let scope = RankingsScope::for_division(kind, gender, MAX_PAGES_PER_EVENT)?;
    let page_scope = RankingsScope::for_division(PAGE_KIND, PAGE_GENDER, MAX_PAGES_PER_EVENT)?;
    let origin = Url::parse(ORIGIN)?;

    let retained: Value = serde_json::from_slice(RETAINED_ROW)?;
    let source: SourceRecord = serde_json::from_value(retained["source"].clone())?;
    let name = CanonicalName::from_source(&source)?
        .context("retained source has no canonical ranking name")?;

    let source_snapshot = SourceSnapshot {
        revision: ACQUISITION_REVISION.to_owned(),
        source_origin: ORIGIN.to_owned(),
        label: "synthetic-rankings-verifier".to_owned(),
        rankings: Some(scope.clone()),
    };
    let source_digest = put(&store, &source_snapshot)?;
    let collection = collection_fingerprint(&scope.revision, &source_digest)?;

    let nav: Value = serde_json::from_str(NAV)?;
    let catalog = EventCatalog::from_nav(&nav, &scope.requested_families, kind, scope.list_id)?;
    let plan =
        catalog
            .clone()
            .into_plan(collection.clone(), scope.projection_grade, &scope.gender)?;
    anyhow::ensure!(
        plan.events.len() == 1,
        "fixture nav must map exactly one event — left={:?} right={:?}",
        &plan.events.len(),
        &1
    );
    let event = plan.events.first().context("plan event")?.clone();

    // Navigation capture: the verifier builds this resource with the
    // projection grade, page one, and the `100m` catalog entry.
    let nav_action = RankingsAction {
        list_id: scope.list_id,
        gender: scope.gender.clone(),
        grade: Some(scope.projection_grade),
        event_short: "100m".to_owned(),
        page: 1,
        capture: RankingsCapture::Navigation,
    };
    let nav_spec = rankings_spec(&origin, nav_action)?;
    let nav_receipt = DocumentReceipt {
        digest: store.put_bytes(NAV.as_bytes())?,
        source_url: nav_spec.semantic_url.clone(),
        http_status: 200,
        media_type: "application/json".to_owned(),
        bytes: u64::try_from(NAV.len())?,
        fetched_at_unix_ms: 1,
        elapsed_ms: 1,
        rankings: Some(RankingPageObservation {
            capture: RankingsCapture::Navigation,
            request_method: "GET".to_owned(),
            request_url: format!(
                "{ORIGIN}api/v1/tfRankings/GetNavInfo?seasonId={}&level=4&gender={}&recordSetId=0&locationId=0&teamId=0&indoor={}",
                scope.source_season_id(),
                scope.gender,
                kind == SeasonKind::Indoor
            ),
            request_body: None,
            next_page: None,
        }),
    };
    let nav_operation = operation('1')?;
    let nav_attempt = captured(&store, &nav_spec, nav_receipt.clone(), &nav_operation)?;
    let catalog_outcome = outcome(nav_receipt, &nav_operation, nav_attempt)?;
    let catalog_ref = put(&store, &catalog)?;
    let plan_ref = put(&store, &plan)?;

    let results_action = RankingsAction {
        list_id: scope.list_id,
        gender: scope.gender.clone(),
        grade: Some(scope.projection_grade),
        event_short: event.short.clone(),
        page: 1,
        capture: RankingsCapture::Results,
    };
    let results_spec = rankings_spec(&origin, results_action)?;
    let body = serde_json::to_string(&results_spec.body().context("rankings body")?)?;
    let page_bytes = PAGE.as_bytes();
    let page_receipt = DocumentReceipt {
        digest: store.put_bytes(page_bytes)?,
        source_url: results_spec.semantic_url.clone(),
        http_status: 200,
        media_type: "application/json".to_owned(),
        bytes: u64::try_from(page_bytes.len())?,
        fetched_at_unix_ms: 1,
        elapsed_ms: 1,
        rankings: Some(RankingPageObservation {
            capture: RankingsCapture::Results,
            request_method: "POST".to_owned(),
            request_url: results_spec.url.to_string(),
            request_body: Some(body),
            next_page: None,
        }),
    };
    let page_operation = operation('2')?;
    let page_attempt = captured(&store, &results_spec, page_receipt.clone(), &page_operation)?;
    let page_outcome = outcome(page_receipt, &page_operation, page_attempt)?;

    let page: Value = serde_json::from_str(PAGE)?;
    let observation = parse_page_response(
        &page,
        &ExpectedPageContext {
            division_id: page_scope.list_id,
            season_id: page_scope.source_season_id(),
            gender: PAGE_GENDER,
            event_short: &event.short,
            event_id: Some(event.event_id),
            is_relay: event.is_relay,
            requested_grade: Some(page_scope.projection_grade),
            page: 1,
        },
    )?;
    let observation_digest = put(&store, &observation)?;
    let checkpoint = RankingsPageCheckpoint {
        revision: scope.revision.clone(),
        collection: collection.clone(),
        event_short: event.short.clone(),
        page: 1,
        previous_checkpoint: None,
        outcome: page_outcome,
        observation_digest,
    };
    let checkpoint_digest = put(&store, &checkpoint)?;
    store.put_rankings_page(&page_index(
        &observation,
        &collection,
        &event.short,
        1,
        &checkpoint_digest,
    )?)?;
    let unique_athletes = store.ranking_collection_stats(&collection)?.unique_athletes;
    let requested = u64::try_from(plan.events.len())?;
    let snapshot = CollectionFinalSnapshot {
        collection: collection.clone(),
        source_snapshot: source_digest.clone(),
        scope: scope.clone(),
        catalog_outcome,
        catalog_ref: Some(catalog_ref),
        plan_ref: Some(plan_ref),
        event_heads: vec![EventHeadRef {
            event_short: event.short.clone(),
            head_checkpoint: Some(checkpoint_digest),
            page_count: 1,
            terminal: true,
        }],
        coverage: CoverageSummary {
            total_requested: requested,
            completed: requested,
            absent_families: catalog
                .absent_families
                .iter()
                .map(|family| family.family.clone())
                .collect(),
        },
        unique_athletes,
    };
    let snapshot_digest = put(&store, &snapshot)?;
    store.seal_rankings(&collection, &snapshot_digest)?;

    let bound = RankingCollectionRef {
        collection: collection.clone(),
        snapshot: snapshot_digest,
    };
    let lookup = store.ranking_name_refs(&collection, &name, MAX_CANDIDATES)?;
    let records = lookup.records.len();
    let discovery = DiscoverySummary {
        job: RowJob {
            workbook: WorkbookDigest::parse(&"c".repeat(64))?,
            snapshot: source_digest,
            source: SourceRowKey::parse(&source.source_key)?,
            rankings: Some(bound.clone()),
        },
        candidate_ids: BTreeSet::new(),
        query_artifacts: Vec::new(),
        complete: true,
        issues: Vec::new(),
        rankings: Some(RankingDiscoveryEvidence {
            canonical_name: Some(name),
            collection: bound,
            lookup,
        }),
    };
    Ok(Frozen {
        store,
        source,
        discovery,
        records,
        _directory: directory,
    })
}

#[test]
fn accepts_a_frozen_indoor_girls_collection() -> anyhow::Result<()> {
    let frozen = frozen(SeasonKind::Indoor, "f")?;
    anyhow::ensure!(
        frozen.records == 1,
        "the frozen index must hold exactly the retained candidate — left={:?} right={:?}",
        &frozen.records,
        &1
    );
    anyhow::ensure!(super::verify(
        &frozen.source,
        &frozen.discovery,
        &frozen.store
    )?);
    Ok(())
}

#[test]
fn rejects_a_scope_whose_season_kind_differs_from_the_frozen_page() -> anyhow::Result<()> {
    // An outdoor seal over the indoor page: the verifier rebuilds the page
    // context from the sealed scope, so the indoor division id cannot pass.
    let frozen = frozen(SeasonKind::Outdoor, "f")?;
    let error = match super::verify(&frozen.source, &frozen.discovery, &frozen.store) {
        Ok(_) => bail!("an outdoor scope must not accept an indoor page"),
        Err(error) => error,
    };
    anyhow::ensure!(
        error.to_string().contains("division ID mismatch"),
        "the rejection must name the division mismatch, got: {error}"
    );
    Ok(())
}

#[test]
fn rejects_a_scope_whose_gender_differs_from_the_frozen_page() -> anyhow::Result<()> {
    // A boys seal over the girls page: the page's request gender must match
    // the sealed scope, not the retained observation.
    let frozen = frozen(SeasonKind::Indoor, "m")?;
    let error = match super::verify(&frozen.source, &frozen.discovery, &frozen.store) {
        Ok(_) => bail!("a boys scope must not accept a girls page"),
        Err(error) => error,
    };
    anyhow::ensure!(
        error.to_string().contains("gender mismatch"),
        "the rejection must name the gender mismatch, got: {error}"
    );
    Ok(())
}
