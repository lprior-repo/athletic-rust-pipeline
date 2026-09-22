use anyhow::{Context, Result};
use athletic_rust_pipeline::{
    domain::{identity::EvidenceDigest, name::CanonicalName},
    store::{
        rankings::{
            RankingCandidateEntry, RankingCandidateKind, RankingPageIndex, RankingSourceRow,
        },
        ArtifactStore, StoreError,
    },
};

fn page(
    store: &ArtifactStore,
    collection: &EvidenceDigest,
    event: &str,
    athletes: &[u64],
) -> Result<RankingPageIndex> {
    let checkpoint = store.put_bytes(&serde_json::to_vec(&(event, athletes))?)?;
    let rows = athletes
        .iter()
        .enumerate()
        .map(|(index, id)| {
            Ok(RankingSourceRow {
                result_id: *id,
                row_number: u64::try_from(index)?
                    .checked_add(1)
                    .context("row overflow")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let candidates = athletes
        .iter()
        .enumerate()
        .map(|(index, id)| {
            Ok(RankingCandidateEntry {
                name: CanonicalName::parse(&format!("Synthetic Athlete {id}"))?,
                athlete_id: (*id).try_into()?,
                kind: RankingCandidateKind::Individual,
                record_index: u32::try_from(index)?,
                result_id: *id,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(RankingPageIndex {
        collection: collection.clone(),
        event_short: event.to_owned(),
        page: 1,
        checkpoint,
        rows,
        candidates,
        rosters: Vec::new(),
    })
}

#[test]
fn exact_page_replay_preserves_counts_before_and_after_sealing() -> Result<()> {
    // Given one published page in a private collection.
    let temp = tempfile::tempdir()?;
    let store = ArtifactStore::open(&temp.path().join("store"))?;
    let collection = store.put_bytes(b"replay collection")?;
    let mut index = page(&store, &collection, "100m", &[1, 2])?;
    store.put_rankings_page(&index)?;
    // When the same page is replayed, including after the collection is sealed.
    store.put_rankings_page(&index)?;
    let snapshot = store.put_bytes(b"sealed snapshot")?;
    store.seal_rankings(&collection, &snapshot)?;
    store.put_rankings_page(&index)?;
    // Then publication is idempotent, while conflicting/new pages remain forbidden.
    let stats = store.ranking_event_stats(&collection, "100m")?;
    {
        let left_value = &(stats.pages, stats.source_results, stats.unique_athletes);
        let right_value = &(1, 2, 2);
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    index.checkpoint = store.put_bytes(b"different checkpoint")?;
    anyhow::ensure!(matches!(
        store.put_rankings_page(&index),
        Err(StoreError::RankingConflict)
    ));
    index.page = 2;
    anyhow::ensure!(matches!(
        store.put_rankings_page(&index),
        Err(StoreError::RankingConflict)
    ));
    {
        let left_value = &(store.ranking_snapshot(&collection)?);
        let right_value = &(Some(snapshot));
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    Ok(())
}

#[test]
fn collection_counts_real_athletes_across_variable_length_event_keys() -> Result<()> {
    // Given two events, with one athlete participating in both.
    let temp = tempfile::tempdir()?;
    let store = ArtifactStore::open(&temp.path().join("store"))?;
    let collection = store.put_bytes(b"multi-event collection")?;
    store.put_rankings_page(&page(&store, &collection, "100m", &[1, 2])?)?;
    store.put_rankings_page(&page(&store, &collection, "110mh", &[1, 3])?)?;
    // When collection statistics scan variable-length event prefixes.
    let stats = store.ranking_collection_stats(&collection)?;
    // Then they count athlete IDs, not bytes from the event name or duplicate appearances.
    anyhow::ensure!(
        stats.unique_athletes == 3,
        "left={:?} right={:?}",
        &stats.unique_athletes,
        &3
    );
    {
        let left_value = &(store
            .ranking_event_stats(&collection, "100m")?
            .unique_athletes);
        let right_value = &2;
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    {
        let left_value = &(store
            .ranking_event_stats(&collection, "110mh")?
            .unique_athletes);
        let right_value = &2;
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    Ok(())
}

#[test]
fn athlete_lookup_preserves_every_record_in_one_checkpoint() -> Result<()> {
    // Given three source-located records for the same athlete on one page.
    let temp = tempfile::tempdir()?;
    let store = ArtifactStore::open(&temp.path().join("store"))?;
    let collection = store.put_bytes(b"multiple records")?;
    let mut index = page(&store, &collection, "100m", &[7, 7, 7])?;
    index.candidates[2].kind = RankingCandidateKind::RelayMember;
    index.candidates[2].record_index = 0;
    store.put_rankings_page(&index)?;
    // When exact publication is replayed and the athlete index is queried.
    store.put_rankings_page(&index)?;
    let lookup = store.ranking_athlete_refs(&collection, 7_u64.try_into()?, 10)?;
    // Then neither repeated individual appearances nor relay membership is lost.
    let records = lookup
        .records
        .iter()
        .map(|record| (record.kind, record.record_index))
        .collect::<Vec<_>>();
    {
        let left_value = &records;
        let right_value = &([
            (RankingCandidateKind::Individual, 0),
            (RankingCandidateKind::Individual, 1),
            (RankingCandidateKind::RelayMember, 0),
        ]);
        anyhow::ensure!(
            left_value == right_value,
            "left={left_value:?} right={right_value:?}"
        );
    }
    anyhow::ensure!(!lookup.truncated);
    Ok(())
}
