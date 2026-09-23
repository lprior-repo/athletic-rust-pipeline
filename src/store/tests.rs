use super::{
    rankings::{
        RankingCandidateEntry, RankingCandidateKind, RankingPageIndex, RankingRosterObservation,
        RankingSourceRow,
    },
    ArtifactStore, StoreError, MAX_BATCH_RECORDS, MAX_DOCUMENT_BYTES,
};
use crate::{
    domain::{
        identity::{AthleteId, EvidenceDigest, SourceRowKey, WorkbookDigest},
        name::CanonicalName,
    },
    model::SourceRecord,
};
use std::collections::BTreeMap;
use tempfile::tempdir;

fn workbook(digit: char) -> WorkbookDigest {
    WorkbookDigest::parse(&digit.to_string().repeat(64)).expect("workbook digest")
}

fn record(sheet: &str, row: u32, marker: &str) -> SourceRecord {
    let mut fields = BTreeMap::new();
    fields.insert("Person First".to_owned(), marker.to_owned());
    SourceRecord {
        source_key: format!("{sheet}:{row}"),
        sheet: sheet.to_owned(),
        excel_row: row,
        fields,
    }
}

fn open_store() -> (tempfile::TempDir, ArtifactStore) {
    let dir = tempdir().expect("temporary store path");
    let store = ArtifactStore::open(&dir.path().join("artifacts")).expect("open store");
    (dir, store)
}

#[test]
fn documents_survive_reopen_and_verify_digest() {
    let (dir, store) = open_store();
    let digest = store.put_bytes(b"fictional source").expect("put document");
    assert_eq!(
        store.get_bytes(&digest).expect("read document"),
        b"fictional source"
    );
    drop(store);
    let reopened = ArtifactStore::open(&dir.path().join("artifacts")).expect("reopen store");
    assert_eq!(
        reopened.get_bytes(&digest).expect("read reopened"),
        b"fictional source"
    );
}

#[test]
fn tampered_document_is_rejected() {
    let (_dir, store) = open_store();
    let digest = store.put_bytes(b"original").expect("put document");
    let key = super::keys::document_key(&digest);
    store
        .inner
        .documents
        .insert(key, b"tampered".to_vec())
        .expect("tamper fixture");
    assert!(matches!(
        store.get_bytes(&digest),
        Err(StoreError::DigestMismatch)
    ));
}

#[test]
fn duplicate_publication_is_idempotent_but_changed_row_conflicts() {
    let (_dir, store) = open_store();
    let wb = workbook('a');
    let first = record("Sheet", 2, "A");
    store
        .put_source_batch(&wb, std::slice::from_ref(&first))
        .expect("first import");
    store
        .put_source_batch(&wb, std::slice::from_ref(&first))
        .expect("duplicate import");
    let changed = record("Sheet", 2, "B");
    assert!(matches!(
        store.put_source_batch(&wb, std::slice::from_ref(&changed)),
        Err(StoreError::SourceConflict)
    ));
}

#[test]
fn a_stale_page_marker_is_cleared_so_the_page_can_be_republished() {
    let (_dir, store) = open_store();
    let collection = store.put_bytes(b"collection").expect("collection digest");
    // Every capture carries the identity of its own evidence.
    let page = |capture: &str, result_id: u64| super::RankingPageIndex {
        collection: collection.clone(),
        event_short: "100m".to_owned(),
        page: 1,
        checkpoint: store
            .put_bytes(capture.as_bytes())
            .expect("checkpoint digest"),
        rows: vec![super::RankingSourceRow {
            result_id,
            row_number: result_id,
        }],
        candidates: Vec::new(),
        rosters: Vec::new(),
    };
    let first = page("first capture", 1);
    store
        .put_rankings_page(&first)
        .expect("first publication");
    store
        .put_rankings_page(&first)
        .expect("identical replay is idempotent");
    let conflicting = page("conflicting capture", 2);
    assert!(matches!(
        store.put_rankings_page(&conflicting),
        Err(StoreError::RankingConflict)
    ));
    assert!(store
        .drop_rankings_page(&collection, "100m", 1)
        .expect("drop the marker left by an unfinished publication"));
    store
        .put_rankings_page(&conflicting)
        .expect("fresh evidence publishes once the stale marker is cleared");
    assert!(
        store
            .drop_rankings_page(&collection, "100m", 1)
            .expect("the republished page owns its marker again"),
        "the republished page must own a marker"
    );
}

#[test]
fn source_rows_are_isolated_and_ordered_by_sheet_then_numeric_row() {
    let (_dir, store) = open_store();
    let first = record("Sheet", 10, "ten");
    let second = record("Sheet", 2, "two");
    let third = record("Other", 2, "other");
    let wb_a = workbook('a');
    let wb_b = workbook('b');
    store
        .put_source_batch(&wb_a, &[first, second, third])
        .expect("import rows");
    store
        .put_source_batch(&wb_b, &[record("Sheet", 2, "isolated")])
        .expect("import other workbook");
    let mut seen = Vec::new();
    store
        .visit_source(&wb_a, |row| {
            seen.push(row.source_key);
            Ok(())
        })
        .expect("visit rows");
    assert_eq!(seen, ["Other:2", "Sheet:2", "Sheet:10"]);
    let key = SourceRowKey::parse("Sheet:2").expect("row key");
    assert_eq!(
        store
            .source_record(&wb_b, &key)
            .expect("read isolated row")
            .map(|row| row.fields["Person First"].clone()),
        Some("isolated".to_owned())
    );
}

#[test]
fn source_pages_resume_after_gaps_without_crossing_sheet_boundaries() {
    let (_dir, store) = open_store();
    let wb = workbook('a');
    let rows = [
        record("Sheet", 2, "first"),
        record("Sheet", 10, "gap"),
        record("Sheet1", 3, "other"),
    ];
    store
        .put_source_batch(&wb, &rows)
        .expect("import sparse rows");
    let first = store
        .source_key_page(&wb, "Sheet", 0, 1)
        .expect("first page");
    assert_eq!(
        first.iter().map(SourceRowKey::as_str).collect::<Vec<_>>(),
        ["Sheet:2"]
    );
    let second = store
        .source_key_page(&wb, "Sheet", 2, 1)
        .expect("next page");
    assert_eq!(
        second.iter().map(SourceRowKey::as_str).collect::<Vec<_>>(),
        ["Sheet:10"]
    );
    assert!(store
        .source_key_page(&wb, "Sheet", 10, 1)
        .expect("end")
        .is_empty());
    assert!(store
        .source_key_page(&workbook('b'), "Sheet", 0, 1)
        .expect("other workbook")
        .is_empty());
    assert!(matches!(
        store.source_key_page(&wb, "Sheet", 0, 0),
        Err(StoreError::InvalidPageLimit)
    ));
}

#[test]
fn bounds_and_visitor_failures_are_returned() {
    let (_dir, store) = open_store();
    let too_large = vec![b'x'; MAX_DOCUMENT_BYTES + 1];
    assert!(matches!(
        store.put_bytes(&too_large),
        Err(StoreError::DocumentTooLarge)
    ));
    let rows = (2..=(MAX_BATCH_RECORDS as u32 + 2))
        .map(|row| record("Sheet", row, "x"))
        .collect::<Vec<_>>();
    let wb = workbook('a');
    assert!(matches!(
        store.put_source_batch(&wb, &rows),
        Err(StoreError::BatchTooLarge)
    ));
    let row = record("Sheet", 2, "x");
    store
        .put_source_batch(&wb, &[row])
        .expect("import callback row");
    let failure = store.visit_source(&wb, |_| Err(StoreError::Visitor));
    assert!(matches!(failure, Err(StoreError::Visitor)));
}

#[test]
fn raw_fields_are_retained_without_normalization() {
    let (_dir, store) = open_store();
    let wb = workbook('a');
    let mut row = record("Sheet", 2, "  exact  ");
    row.fields
        .insert("Unrecognized Header".to_owned(), "  keep me  ".to_owned());
    store
        .put_source_batch(&wb, &[row])
        .expect("import raw fields");
    let key = SourceRowKey::parse("Sheet:2").expect("row key");
    let stored = store
        .source_record(&wb, &key)
        .expect("read raw fields")
        .expect("present");
    assert_eq!(stored.fields["Person First"], "  exact  ");
    assert_eq!(stored.fields["Unrecognized Header"], "  keep me  ");
}

#[test]
fn absent_artifacts_are_distinct_from_corruption() {
    let (_dir, store) = open_store();
    let digest = EvidenceDigest::parse(&"b".repeat(64)).expect("digest");
    assert!(matches!(
        store.get_bytes(&digest),
        Err(StoreError::MissingArtifact)
    ));
}

#[cfg(unix)]
#[test]
fn rejects_existing_root_with_group_or_other_access() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempdir().expect("temporary parent");
    let root = dir.path().join("store");
    std::fs::create_dir(&root).expect("create root");
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755))
        .expect("set fixture permissions");
    assert!(matches!(
        ArtifactStore::open(&root),
        Err(StoreError::InsecureRoot)
    ));
}

fn athlete(id: u64) -> AthleteId {
    AthleteId::try_from(id).expect("athlete id")
}

/// One capture of page 1 of one event: row N is result N at position N, and
/// candidate N is an individual result for athlete N. `capture` distinguishes
/// captures of the same page, because the checkpoint is the capture's identity.
fn page(
    store: &ArtifactStore,
    collection: &EvidenceDigest,
    capture: &str,
    athletes: &[u64],
) -> RankingPageIndex {
    let checkpoint = store
        .put_bytes(&serde_json::to_vec(&(capture, athletes)).expect("page fixture"))
        .expect("checkpoint digest");
    RankingPageIndex {
        collection: collection.clone(),
        event_short: "100m".to_owned(),
        page: 1,
        checkpoint,
        rows: athletes
            .iter()
            .enumerate()
            .map(|(index, id)| RankingSourceRow {
                result_id: *id,
                row_number: u64::try_from(index).expect("row position").saturating_add(1),
            })
            .collect(),
        candidates: athletes
            .iter()
            .enumerate()
            .map(|(index, id)| RankingCandidateEntry {
                name: CanonicalName::parse(&format!("Athlete {id}")).expect("candidate name"),
                athlete_id: athlete(*id),
                kind: RankingCandidateKind::Individual,
                record_index: u32::try_from(index).expect("record index"),
                result_id: *id,
            })
            .collect(),
        rosters: Vec::new(),
    }
}

#[test]
fn sealed_collection_refuses_page_drop() {
    let (_dir, store) = open_store();
    let collection = store.put_bytes(b"sealed collection").expect("collection");
    let index = page(&store, &collection, "a", &[1, 2]);
    store.put_rankings_page(&index).expect("publish the page");
    let snapshot = store.put_bytes(b"sealed snapshot").expect("snapshot");
    store.seal_rankings(&collection, &snapshot).expect("seal");

    assert!(matches!(
        store.drop_rankings_page(&collection, "100m", 1),
        Err(StoreError::RankingConflict)
    ));

    // The page the seal certifies still reads as published, and only an exact
    // replay of it is accepted.
    assert_eq!(
        store.ranking_snapshot(&collection).expect("seal"),
        Some(snapshot)
    );
    let stats = store
        .ranking_event_stats(&collection, "100m")
        .expect("event stats");
    assert_eq!((stats.pages, stats.source_results), (1, 2));
    assert!(store.put_rankings_page(&index).is_ok());
}

#[test]
fn dropped_page_marker_survives_a_reopen_and_a_crash() {
    let dir = tempdir().expect("temporary store path");
    let path = dir.path().join("artifacts");
    let collection = {
        let store = ArtifactStore::open(&path).expect("open store");
        let collection = store.put_bytes(b"crash collection").expect("collection");
        store
            .put_rankings_page(&page(&store, &collection, "a", &[1]))
            .expect("publish capture a");
        assert!(store
            .drop_rankings_page(&collection, "100m", 1)
            .expect("drop capture a's marker"));
        collection
    };

    // A clean reopen already observes the removal.
    let reopened = ArtifactStore::open(&path).expect("reopen store");
    assert!(
        !reopened
            .drop_rankings_page(&collection, "100m", 1)
            .expect("probe the marker after a reopen"),
        "the dropped marker must not be observable after a reopen"
    );
    reopened
        .put_rankings_page(&page(&reopened, &collection, "b", &[2]))
        .expect("publish capture b");
    drop(reopened);

    // A crash after the same recovery maneuver must observe it too: the child
    // drops capture b's marker and aborts without unwinding, so nothing it wrote
    // reaches disk unless the drop committed it durably.
    let child = std::process::Command::new(std::env::current_exe().expect("test binary"))
        .args([
            "--exact",
            "store::tests::crash_child_drops_the_page_marker_then_aborts",
            "--ignored",
            "--nocapture",
        ])
        .env("CRASH_STORE_DIR", &path)
        .env("CRASH_COLLECTION", collection.as_str())
        .output()
        .expect("spawn the crash child");
    assert!(
        !child.status.success(),
        "the crash child must die without a clean exit"
    );

    let recovered = ArtifactStore::open(&path).expect("reopen after the crash");
    assert!(
        !recovered
            .drop_rankings_page(&collection, "100m", 1)
            .expect("probe the marker after a crash"),
        "the dropped marker must not come back after a crash"
    );

    // The same commit that clears the marker revokes the capture it certified, so
    // the crash must not have lost the revocation either: rebuilding capture b
    // with other content is still refused. (An abort preserves whatever reached
    // the OS, so the marker probe above cannot tell a durable commit from a
    // buffered write; the revocation is what a lost drop-commit drops first.)
    let mut republished = page(&recovered, &collection, "b", &[2]);
    republished.rows[0].row_number = 7;
    assert!(
        matches!(
            recovered.put_rankings_page(&republished),
            Err(StoreError::RankingConflict)
        ),
        "the revoked capture must stay revoked after a crash"
    );
}

#[test]
#[ignore = "child of dropped_page_marker_survives_a_reopen_and_a_crash"]
fn crash_child_drops_the_page_marker_then_aborts() {
    let dir = std::env::var("CRASH_STORE_DIR").expect("crash child store directory");
    let raw = std::env::var("CRASH_COLLECTION").expect("crash child collection");
    let collection = EvidenceDigest::parse(&raw).expect("collection digest");
    let store = ArtifactStore::open(std::path::Path::new(&dir)).expect("open store in the child");
    store
        .drop_rankings_page(&collection, "100m", 1)
        .expect("drop the marker in the crash child");
    std::process::abort();
}

#[test]
fn malformed_reference_values_are_corruption_not_absence() {
    let (_dir, store) = open_store();
    let collection = store.put_bytes(b"reference collection").expect("collection");
    let index = page(&store, &collection, "a", &[7, 8]);
    store.put_rankings_page(&index).expect("publish the page");

    let name = CanonicalName::parse("Athlete 7").expect("name");
    let athlete = athlete(7);
    let record = super::RankingRecordRef {
        athlete_id: athlete,
        checkpoint: index.checkpoint.clone(),
        kind: RankingCandidateKind::Individual,
        record_index: 0,
    };
    // The intact index answers both lookups before it is corrupted.
    assert_eq!(
        store
            .ranking_name_refs(&collection, &name, 10)
            .expect("name lookup")
            .records
            .len(),
        1
    );
    assert_eq!(
        store
            .ranking_athlete_refs(&collection, athlete, 10)
            .expect("athlete lookup")
            .records
            .len(),
        1
    );
    let name_key = super::rankings::keys::name_ref_key(
        &collection,
        name.as_str(),
        athlete,
        &index.checkpoint,
        RankingCandidateKind::Individual,
        0,
    )
    .expect("name ref key");
    let athlete_key =
        super::rankings::keys::athlete_ref_key(&collection, &record).expect("athlete ref key");
    store
        .inner
        .rankings
        .insert(name_key, b"{ not a record ref".to_vec())
        .expect("corrupt the name ref");
    store
        .inner
        .rankings
        .insert(athlete_key, b"{ not a record ref".to_vec())
        .expect("corrupt the athlete ref");

    assert!(matches!(
        store.ranking_name_refs(&collection, &name, 10),
        Err(StoreError::CorruptData)
    ));
    assert!(matches!(
        store.ranking_athlete_refs(&collection, athlete, 10),
        Err(StoreError::CorruptData)
    ));
}

#[test]
fn page_write_refuses_values_the_stats_decoders_call_corrupt() {
    let (_dir, store) = open_store();
    let collection = store.put_bytes(b"validation collection").expect("collection");
    let base = page(&store, &collection, "a", &[1, 2]);
    let marker = super::rankings::keys::page_marker_key(&collection, "100m", 1)
        .expect("page marker key");

    let zero_result_id = {
        let mut index = base.clone();
        index.rows[0].result_id = 0;
        index
    };
    let zero_row_number = {
        let mut index = base.clone();
        index.rows[0].row_number = 0;
        index
    };
    let zero_candidate_result = {
        let mut index = base.clone();
        index.candidates[0].result_id = 0;
        index
    };
    let candidate_outside_page = {
        let mut index = base.clone();
        index.candidates[0].result_id = 99;
        index
    };
    let zero_roster_result = {
        let mut index = base.clone();
        index.rosters = vec![RankingRosterObservation {
            result_id: 0,
            present: true,
        }];
        index
    };
    let roster_outside_page = {
        let mut index = base.clone();
        index.rosters = vec![RankingRosterObservation {
            result_id: 99,
            present: true,
        }];
        index
    };
    let contradictory_roster = {
        let mut index = base.clone();
        index.rosters = vec![
            RankingRosterObservation {
                result_id: 1,
                present: true,
            },
            RankingRosterObservation {
                result_id: 1,
                present: false,
            },
        ];
        index
    };

    for (label, index) in [
        ("zero result id", zero_result_id),
        ("zero row number", zero_row_number),
        ("zero candidate result id", zero_candidate_result),
        ("candidate result outside the page", candidate_outside_page),
        ("zero roster result id", zero_roster_result),
        ("roster result outside the page", roster_outside_page),
        ("contradictory roster presence", contradictory_roster),
    ] {
        assert!(
            matches!(
                store.put_rankings_page(&index),
                Err(StoreError::InvalidRankingInput)
            ),
            "the writer must refuse {label}"
        );
        assert!(
            store
                .inner
                .rankings
                .get(&marker)
                .expect("marker probe")
                .is_none(),
            "refusing {label} must write nothing"
        );
    }

    // The unmodified page still publishes.
    assert!(store.put_rankings_page(&base).is_ok());
}

#[test]
fn an_abandoned_capture_is_stored_but_invisible_to_every_reader() {
    let (_dir, store) = open_store();
    let collection = store.put_bytes(b"republication collection").expect("collection");
    let abandoned = {
        let mut index = page(&store, &collection, "a", &[100]);
        // Position 5 and a missing roster make every event statistic move if the
        // abandoned capture is ever counted.
        index.rows[0].row_number = 5;
        index.rosters = vec![RankingRosterObservation {
            result_id: 100,
            present: false,
        }];
        index
    };
    store
        .put_rankings_page(&abandoned)
        .expect("publish capture a");
    assert!(store
        .drop_rankings_page(&collection, "100m", 1)
        .expect("drop capture a's marker"));
    let accepted = page(&store, &collection, "b", &[101]);
    store
        .put_rankings_page(&accepted)
        .expect("publish capture b");

    // Capture a's index keys are still stored: identity, not deletion, is what
    // hides them.
    let stored_keys = [
        super::rankings::keys::presence_source_result(
            &collection,
            "100m",
            &abandoned.checkpoint,
            100,
        )
        .expect("source result key"),
        super::rankings::keys::presence_row_position(
            &collection,
            "100m",
            &abandoned.checkpoint,
            100,
            5,
        )
        .expect("row position key"),
        super::rankings::keys::presence_eligible_individual(
            &collection,
            "100m",
            &abandoned.checkpoint,
            100,
            athlete(100),
        )
        .expect("eligible individual key"),
        super::rankings::keys::presence_roster_missing(
            &collection,
            "100m",
            &abandoned.checkpoint,
            100,
        )
        .expect("roster missing key"),
        super::rankings::keys::presence_event_athlete(
            &collection,
            "100m",
            &abandoned.checkpoint,
            athlete(100),
        )
        .expect("event athlete key"),
        super::rankings::keys::name_ref_key(
            &collection,
            CanonicalName::parse("Athlete 100")
                .expect("candidate name")
                .as_str(),
            athlete(100),
            &abandoned.checkpoint,
            RankingCandidateKind::Individual,
            0,
        )
        .expect("name ref key"),
        super::rankings::keys::athlete_ref_key(
            &collection,
            &super::RankingRecordRef {
                athlete_id: athlete(100),
                checkpoint: abandoned.checkpoint.clone(),
                kind: RankingCandidateKind::Individual,
                record_index: 0,
            },
        )
        .expect("athlete ref key"),
    ];
    for key in &stored_keys {
        assert!(
            store
                .inner
                .rankings
                .get(key)
                .expect("stored key probe")
                .is_some(),
            "the abandoned capture's keys stay stored"
        );
    }

    // No reader returns any of it.
    let stats = store
        .ranking_event_stats(&collection, "100m")
        .expect("event stats");
    assert_eq!(stats.pages, 1);
    assert_eq!(stats.source_results, 1);
    assert_eq!(stats.row_positions, 1);
    assert_eq!(stats.max_row_position, 1);
    assert_eq!(stats.grade11_individual_results, 1);
    assert_eq!(stats.grade11_relay_member_results, 0);
    assert_eq!(stats.unique_athletes, 1);
    assert_eq!(stats.unresolved_roster_results, 0);
    assert_eq!(
        store
            .ranking_collection_stats(&collection)
            .expect("collection stats")
            .unique_athletes,
        1
    );
    assert!(store
        .ranking_name_refs(
            &collection,
            &CanonicalName::parse("Athlete 100").expect("name"),
            10
        )
        .expect("abandoned name lookup")
        .records
        .is_empty());
    assert!(store
        .ranking_athlete_refs(&collection, athlete(100), 10)
        .expect("abandoned athlete lookup")
        .records
        .is_empty());
    let accepted_lookup = store
        .ranking_athlete_refs(&collection, athlete(101), 10)
        .expect("accepted athlete lookup");
    assert_eq!(accepted_lookup.records.len(), 1);
    assert_eq!(accepted_lookup.records[0].checkpoint, accepted.checkpoint);
}

#[test]
fn a_dropped_captures_checkpoint_is_not_reused_for_other_content() {
    let (_dir, store) = open_store();
    let collection = store.put_bytes(b"revoked collection").expect("collection");
    let abandoned = page(&store, &collection, "a", &[1]);
    store
        .put_rankings_page(&abandoned)
        .expect("publish capture a");
    assert!(store
        .drop_rankings_page(&collection, "100m", 1)
        .expect("drop capture a's marker"));

    // The same capture republished byte for byte writes the same keys, which
    // cannot mix anything with the abandoned ones.
    store
        .put_rankings_page(&abandoned)
        .expect("an identical republication is admitted");
    assert!(store
        .drop_rankings_page(&collection, "100m", 1)
        .expect("drop the identical republication"));

    // Other content under the dropped capture's checkpoint would be counted
    // beside the stored keys of that capture.
    let mut other = abandoned.clone();
    other.rows[0].row_number = 2;
    assert!(matches!(
        store.put_rankings_page(&other),
        Err(StoreError::RankingConflict)
    ));
}

#[test]
fn fresh_store_stamps_the_ranking_key_revision() {
    let dir = tempdir().expect("temporary store path");
    let path = dir.path().join("artifacts");
    {
        let store = ArtifactStore::open(&path).expect("open store");
        assert_eq!(
            store
                .inner
                .meta
                .get(super::rankings::common::RANKING_KEY_REVISION_META_KEY)
                .expect("revision probe")
                .map(|value| value.as_ref().to_vec()),
            Some(
                super::rankings::common::RANKING_KEY_REVISION
                    .as_bytes()
                    .to_vec()
            )
        );
    }
    let reopened = ArtifactStore::open(&path).expect("reopen store");
    assert_eq!(
        reopened
            .inner
            .meta
            .get(super::rankings::common::RANKING_KEY_REVISION_META_KEY)
            .expect("revision probe")
            .as_ref()
            .map(|value| value.as_ref().to_vec()),
        Some(
            super::rankings::common::RANKING_KEY_REVISION
                .as_bytes()
                .to_vec()
        )
    );
}

#[test]
fn ranking_keys_without_a_revision_marker_are_refused() {
    let dir = tempdir().expect("temporary store path");
    let path = dir.path().join("artifacts");
    {
        let store = ArtifactStore::open(&path).expect("open store");
        let collection = store.put_bytes(b"legacy collection").expect("collection");
        store
            .put_rankings_page(&page(&store, &collection, "a", &[1]))
            .expect("publish the page");
        // A keyspace holding ranking keys and no revision marker is a store from
        // the previous key encoding, whose keys cannot be read or migrated.
        store
            .inner
            .meta
            .remove(super::rankings::common::RANKING_KEY_REVISION_META_KEY)
            .expect("clear the revision marker");
    }
    assert!(matches!(
        ArtifactStore::open(&path),
        Err(StoreError::CorruptData)
    ));
}

#[test]
fn ranking_keys_under_another_revision_are_refused() {
    let dir = tempdir().expect("temporary store path");
    let path = dir.path().join("artifacts");
    {
        let store = ArtifactStore::open(&path).expect("open store");
        store
            .inner
            .meta
            .insert(
                super::rankings::common::RANKING_KEY_REVISION_META_KEY,
                b"1".to_vec(),
            )
            .expect("stamp another revision");
    }
    assert!(matches!(
        ArtifactStore::open(&path),
        Err(StoreError::CorruptData)
    ));
}
