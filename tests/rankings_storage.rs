use anyhow::{Context, Result};
use athletic_rust_pipeline::{
    domain::{identity::EvidenceDigest, name::CanonicalName},
    store::{
        rankings::{
            RankingCandidateEntry, RankingCandidateKind, RankingPageIndex,
            RankingRosterObservation, RankingSourceRow,
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

#[test]
fn a_republished_page_hides_the_abandoned_capture() -> Result<()> {
    // Given a page whose first capture the run abandoned before it sealed.
    let temp = tempfile::tempdir()?;
    let store = ArtifactStore::open(&temp.path().join("store"))?;
    let collection = store.put_bytes(b"republication collection")?;
    let mut abandoned = page(&store, &collection, "100m", &[100])?;
    // Position 5 and a missing roster move every event statistic if the
    // abandoned capture is counted.
    abandoned.rows[0].row_number = 5;
    abandoned.rosters = vec![RankingRosterObservation {
        result_id: 100,
        present: false,
    }];
    store.put_rankings_page(&abandoned)?;
    anyhow::ensure!(
        store.drop_rankings_page(&collection, "100m", 1)?,
        "the abandoned capture owned a marker"
    );

    // When the retried step publishes its own capture of the same page.
    let accepted = page(&store, &collection, "100m", &[101])?;
    store.put_rankings_page(&accepted)?;

    // Then counts, lookups and statistics see capture b alone.
    let stats = store.ranking_event_stats(&collection, "100m")?;
    for (label, observed, expected) in [
        ("pages", stats.pages, 1),
        ("source results", stats.source_results, 1),
        ("row positions", stats.row_positions, 1),
        ("max row position", stats.max_row_position, 1),
        (
            "grade 11 individual results",
            stats.grade11_individual_results,
            1,
        ),
        ("unique athletes", stats.unique_athletes, 1),
        (
            "unresolved roster results",
            stats.unresolved_roster_results,
            0,
        ),
    ] {
        anyhow::ensure!(
            observed == expected,
            "{label}: observed={observed} expected={expected}"
        );
    }
    let collection_stats = store.ranking_collection_stats(&collection)?;
    anyhow::ensure!(
        collection_stats.unique_athletes == 1,
        "collection unique athletes: observed={} expected=1",
        collection_stats.unique_athletes
    );

    let abandoned_name = CanonicalName::parse("Synthetic Athlete 100")?;
    let abandoned_lookup = store.ranking_name_refs(&collection, &abandoned_name, 10)?;
    anyhow::ensure!(
        abandoned_lookup.records.is_empty(),
        "the abandoned capture's name resolved to {} records",
        abandoned_lookup.records.len()
    );
    let accepted_name = CanonicalName::parse("Synthetic Athlete 101")?;
    let accepted_lookup = store.ranking_name_refs(&collection, &accepted_name, 10)?;
    anyhow::ensure!(
        accepted_lookup.records.len() == 1,
        "the accepted capture's name resolved to {} records",
        accepted_lookup.records.len()
    );
    anyhow::ensure!(
        accepted_lookup.records[0].checkpoint == accepted.checkpoint,
        "the name lookup returned another capture's checkpoint"
    );

    let abandoned_athlete = store.ranking_athlete_refs(&collection, 100_u64.try_into()?, 10)?;
    anyhow::ensure!(
        abandoned_athlete.records.is_empty(),
        "the abandoned capture's athlete resolved to {} records",
        abandoned_athlete.records.len()
    );
    let accepted_athlete = store.ranking_athlete_refs(&collection, 101_u64.try_into()?, 10)?;
    anyhow::ensure!(
        accepted_athlete.records.len() == 1,
        "the accepted capture's athlete resolved to {} records",
        accepted_athlete.records.len()
    );
    anyhow::ensure!(
        accepted_athlete.records[0].checkpoint == accepted.checkpoint,
        "the athlete lookup returned another capture's checkpoint"
    );
    Ok(())
}

#[test]
fn a_sealed_collection_refuses_mutation_after_republishing() -> Result<()> {
    // Given a page republished after its first capture was abandoned, and sealed.
    let temp = tempfile::tempdir()?;
    let store = ArtifactStore::open(&temp.path().join("store"))?;
    let collection = store.put_bytes(b"sealed republication")?;
    store.put_rankings_page(&page(&store, &collection, "100m", &[100])?)?;
    anyhow::ensure!(
        store.drop_rankings_page(&collection, "100m", 1)?,
        "the abandoned capture owned a marker"
    );
    let accepted = page(&store, &collection, "100m", &[101])?;
    store.put_rankings_page(&accepted)?;
    let snapshot = store.put_bytes(b"sealed snapshot")?;
    store.seal_rankings(&collection, &snapshot)?;

    // Then neither the marker nor the page can be mutated any more.
    anyhow::ensure!(
        matches!(
            store.drop_rankings_page(&collection, "100m", 1),
            Err(StoreError::RankingConflict)
        ),
        "a sealed collection refused nothing"
    );
    let third = page(&store, &collection, "100m", &[102])?;
    anyhow::ensure!(
        matches!(
            store.put_rankings_page(&third),
            Err(StoreError::RankingConflict)
        ),
        "a sealed collection admitted a new capture"
    );
    anyhow::ensure!(
        store.ranking_snapshot(&collection)? == Some(snapshot),
        "the seal changed"
    );
    let stats = store.ranking_event_stats(&collection, "100m")?;
    anyhow::ensure!(
        stats.source_results == 1 && stats.unique_athletes == 1,
        "the seal left the accepted slice wrong: source_results={} unique_athletes={}",
        stats.source_results,
        stats.unique_athletes
    );
    Ok(())
}
