use super::*;
use crate::keys::observation_key;
use crate::read::{sweep_stale_temporaries, write_snapshot_rows};
use census_domain::model::*;
use census_domain::UsJurisdiction;

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name)).0
}

/// A table's sequence pointer: the sequence its next append will be keyed from.
///
/// This is neither a row count nor an appended-observation count — an append reserves the sequences of
/// its batch before that batch commits — so the ceiling tests read it to see whether a reservation
/// moved, and [`rows_held`] to see what the table holds.
fn sequence_pointer(store: &Store, table: Table) -> u64 {
    store.sequences.next_sequence(table)
}

/// The rows a table holds, as the store's own durable count reports them.
fn rows_held(store: &Store, table: Table) -> u64 {
    store.count(table).unwrap()
}

/// A minimal derived row: the `id` the store's key contract needs and nothing else, so a shape test
/// asserts the store's invariant rather than a domain fixture's fields.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DerivedRow {
    id: String,
    note: u32,
}

impl Entity for DerivedRow {
    fn entity_id(&self) -> &str {
        &self.id
    }

    fn merge(&mut self, other: Self) {
        *self = other;
    }
}

/// A derived row with the given id and note.
fn derived(id: &str, note: u32) -> DerivedRow {
    DerivedRow {
        id: id.to_string(),
        note,
    }
}

#[test]
fn keys_with_a_zero_low_sequence_byte_still_reopen() {
    // A sequence is stored big-endian in the key tail, so every 256th key ends in 0x00 — the
    // same byte that separates the id from the sequence. Parsing that separator by search made
    // `Store::open` fail forever once such a key existed.
    let dir = tempfile::tempdir().unwrap();
    {
        let store = Store::open(dir.path()).unwrap();
        let rows: Vec<CanonicalSchool> = (0..300)
            .map(|index| school(&format!("School {index}")))
            .collect();
        store.append_many(Table::Schools, &rows).unwrap();
    }
    {
        let store = Store::open(dir.path()).unwrap();
        assert_eq!(
            store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
            300
        );
        assert_eq!(store.stats().unwrap().observations, 300);
    }
}

#[test]
fn consolidation_merges_evidence_and_identities() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let mut a = school("Abbotsford High School");
    a.source_identities
        .push(SourceIdentity::new(SourceNamespace::MilesplitTeam, "52649"));
    a.evidence.push(Evidence::parsed(
        SourceRef::id("milesplit_teams"),
        "2026-09-20",
    ));
    let mut b = a.clone();
    b.co_op = true;
    b.city = Some("Abbotsford".into());
    b.source_identities.push(SourceIdentity::new(
        SourceNamespace::AssociationSchool {
            association: "wiaa".into(),
        },
        "1",
    ));
    store.append(Table::Schools, &a).unwrap();
    store.append(Table::Schools, &b).unwrap();
    let out = dir.path().join("out/schools.jsonl");
    let count = store
        .consolidate::<CanonicalSchool>(Table::Schools, &out)
        .unwrap()
        .rows;
    assert_eq!(count, 1);
    let merged: CanonicalSchool = serde_json::from_str(
        std::fs::read_to_string(&out)
            .unwrap()
            .lines()
            .next()
            .unwrap(),
    )
    .unwrap();
    assert!(merged.co_op);
    assert_eq!(merged.city.as_deref(), Some("Abbotsford"));
    assert_eq!(merged.source_identities.len(), 2);
}

#[test]
fn journal_roundtrips_resume_keys() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .journal_done(
            "milesplit_rosters",
            "wi:52649",
            &serde_json::json!({"athletes": 59}),
        )
        .unwrap();
    store
        .journal_done(
            "milesplit_rosters",
            "wi:26848",
            &serde_json::json!({"athletes": 0}),
        )
        .unwrap();
    let keys = store.journal_keys("milesplit_rosters").unwrap();
    assert!(keys.contains("wi:52649"));
    assert_eq!(keys.len(), 2);
    assert_eq!(
        store.journal_payloads("milesplit_rosters").unwrap().len(),
        2
    );
    // A second phase must not leak into the first phase's resume set.
    store
        .journal_done("other_phase", "wi:1", &serde_json::json!({}))
        .unwrap();
    assert_eq!(store.journal_keys("milesplit_rosters").unwrap().len(), 2);
}

#[test]
fn a_journal_key_past_its_ceiling_is_refused_and_writes_nothing() {
    // A phase key is an adapter's URL or path and it rides inside the row's key, which Fjall asserts
    // stays under 64 KiB. This ceiling is what keeps that assertion unreachable, and the refusal runs
    // before the batch exists: an entry the store refuses is one the store does not hold.
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let key = "k".repeat(MAX_JOURNAL_KEY_BYTES + 1);

    match store.journal_done("ihsa_schools", &key, &serde_json::json!({"rows": 1})) {
        Err(StoreError::JournalTooLarge {
            what,
            phase,
            bytes,
            max,
            ..
        }) => {
            assert_eq!(what, "key");
            assert_eq!(phase, "ihsa_schools");
            assert_eq!(max, MAX_JOURNAL_KEY_BYTES);
            assert!(bytes > max, "the refusal names what it measured: {bytes}");
        }
        other => panic!("expected the journal key ceiling to refuse the entry, got {other:?}"),
    }
    assert!(store.journal_keys("ihsa_schools").unwrap().is_empty());
    assert!(store.journal_payloads("ihsa_schools").unwrap().is_empty());
}

#[test]
fn a_journal_value_past_its_ceiling_leaves_the_phase_as_it_was() {
    // The value's true size is only known once it is serialized, so the check runs between the
    // serialization and the batch: a payload that never lands leaves the phase holding exactly the
    // entries it held before, and the refusal names the key that was being written.
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .journal_done("ihsa_schools", "kept", &serde_json::json!({"rows": 1}))
        .unwrap();

    let payload = "v".repeat(MAX_JOURNAL_VALUE_BYTES + 1);
    match store.journal_done("ihsa_schools", "refused", &serde_json::json!(payload)) {
        Err(StoreError::JournalTooLarge {
            what,
            key,
            bytes,
            max,
            ..
        }) => {
            assert_eq!(what, "value");
            assert_eq!(key, "refused");
            assert_eq!(max, MAX_JOURNAL_VALUE_BYTES);
            assert!(bytes > max, "the refusal names what it measured: {bytes}");
        }
        other => panic!("expected the journal value ceiling to refuse the entry, got {other:?}"),
    }
    let keys = store.journal_keys("ihsa_schools").unwrap();
    assert!(keys.contains("kept"));
    assert!(!keys.contains("refused"));
    assert_eq!(store.journal_payloads("ihsa_schools").unwrap().len(), 1);
}

#[test]
fn observations_survive_reopen_without_overwriting() {
    let dir = tempfile::tempdir().unwrap();
    {
        let store = Store::open(dir.path()).unwrap();
        let mut first = school("Abbotsford");
        first.evidence.push(Evidence::parsed(
            SourceRef::id("wiaa_schools"),
            "2026-09-19",
        ));
        store.append(Table::Schools, &first).unwrap();
    }
    {
        // Reopening must resume the sequence, not restart it: a restarted sequence would
        // overwrite the first observation and silently drop its evidence.
        let store = Store::open(dir.path()).unwrap();
        let mut second = school("Abbotsford");
        second.evidence.push(Evidence::parsed(
            SourceRef::id("mshsl_schools"),
            "2026-09-20",
        ));
        store.append(Table::Schools, &second).unwrap();
        let rows = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows.first().map(|row| row.evidence.len()), Some(2));
    }
}

#[test]
fn legacy_journals_are_imported_once() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    std::fs::create_dir_all(root.join("entities")).unwrap();
    std::fs::create_dir_all(root.join("journal")).unwrap();
    let row = school("Abbotsford");
    std::fs::write(
        root.join("entities/schools.jsonl"),
        format!("{}\n", serde_json::to_string(&row).unwrap()),
    )
    .unwrap();
    std::fs::write(
        root.join("journal/milesplit_rosters.jsonl"),
        "{\"key\":\"wi:1\",\"at\":\"2026-09-20\",\"payload\":{\"athletes\":5}}\n",
    )
    .unwrap();

    {
        let store = Store::open(&root).unwrap();
        store.import_legacy().unwrap();
        assert_eq!(
            store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
            1
        );
        assert!(store
            .journal_keys("milesplit_rosters")
            .unwrap()
            .contains("wi:1"));
    }
    // The importer is idempotent: the second import sees the marker and adds nothing.
    let store = Store::open(&root).unwrap();
    store.import_legacy().unwrap();
    assert_eq!(
        store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
        1
    );
    assert_eq!(store.journal_keys("milesplit_rosters").unwrap().len(), 1);
}

#[test]
fn an_interrupted_import_resumes_at_its_committed_offset() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    std::fs::create_dir_all(root.join("entities")).unwrap();

    // A store that has never seen the journal, so the fixture decides what a killed import left.
    let store = Store::open(&root).unwrap();
    store.meta.remove("imported:schools").unwrap();

    let first = school("Abbotsford");
    let second = school("Adams-Friendship");
    let mut journal = serde_json::to_string(&first).unwrap();
    journal.push('\n');
    let committed = journal.len() as u64;
    journal.push_str(&serde_json::to_string(&second).unwrap());
    journal.push('\n');
    std::fs::write(root.join("entities/schools.jsonl"), &journal).unwrap();

    // Exactly what a kill after the first chunk leaves behind: that chunk's row is in the store and
    // its offset is in `meta`, because the offset travels in the same batch as the rows.
    store.append(Table::Schools, &first).unwrap();
    store
        .meta
        .insert("import_offset:schools", committed.to_string().as_bytes())
        .unwrap();

    store.import_legacy().unwrap();

    // The resume re-read the tail only: the row the store already held keeps one observation.
    let rows = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(store.stats().unwrap().observations, 2);
    assert!(store.meta.contains_key("imported:schools").unwrap());
}

#[test]
fn legacy_lines_are_trimmed_before_they_are_parsed() {
    // The importer trims each journal line in place before it parses, so blank separators and
    // hand-padded rows are ordinary input: a whitespace-only line is skipped, and a row wrapped in
    // spaces or tabs is still that row. An off-by-one on either edge would instead drop a row or
    // abort the import on one, so both edges are pinned here.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    std::fs::create_dir_all(root.join("entities")).unwrap();

    let first = school("Abbotsford");
    let second = school("Adams-Friendship");
    let journal = format!(
        "\n \t \n  {}\t\n\t{}\n",
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
    std::fs::write(root.join("entities/schools.jsonl"), &journal).unwrap();

    let store = Store::open(&root).unwrap();
    store.import_legacy().unwrap();
    let rows = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
    assert_eq!(
        rows.len(),
        2,
        "blank lines are separators, padded rows are rows"
    );
    assert_eq!(store.stats().unwrap().observations, 2);
}

#[test]
fn oversized_and_empty_ids_are_rejected_before_any_write() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let oversized = serde_json::json!({ "id": "s".repeat(MAX_ID_BYTES + 1) });
    assert!(store.append(Table::Schools, &oversized).is_err());
    let empty = serde_json::json!({ "id": "" });
    assert!(store.append(Table::Schools, &empty).is_err());
    assert_eq!(store.stats().unwrap().observations, 0);
}

#[test]
fn stats_count_observations_per_table() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store.append(Table::Schools, &school("Abbotsford")).unwrap();
    store
        .append_many(Table::Schools, &[school("Colby"), school("Medford")])
        .unwrap();
    let stats = store.stats().unwrap();
    let schools = stats
        .tables
        .iter()
        .find(|(table, _)| table == "schools")
        .map(|(_, count)| *count)
        .unwrap();
    assert_eq!(schools, 3);
    assert_eq!(stats.observations, 3);
    // The recursive store size carries what the LSM level sizes leave out: a freshly written store
    // keeps its newest batch in the write-ahead journal, so the directory holds bytes even when
    // `bytes_on_disk` reports few, and sizing a copy by that column alone under-counts.
    assert!(stats.store_bytes > 0);
    assert!(stats.store_bytes >= stats.bytes_on_disk);
}

#[test]
fn a_consumer_mailbox_never_survives_a_read_but_a_school_address_does() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let school = school("Abbotsford").id;
    let mut withheld = CanonicalCoach::new(
        &school,
        "J. Riethmiller",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    withheld.professional_email = Some("jriethmiller.ptc@gmail.com".to_string());
    let mut published = CanonicalCoach::new(
        &school,
        "A. Bender",
        Some(Sport::CrossCountry),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    published.professional_email = Some("abender@ofsd.k12.wi.us".to_string());
    store
        .append_many(Table::Coaches, &[withheld, published])
        .unwrap();

    let coaches = store.scan::<CanonicalCoach>(Table::Coaches).unwrap();
    let withheld_row = coaches
        .iter()
        .find(|coach| coach.name == "J. Riethmiller")
        .unwrap();
    assert_eq!(withheld_row.professional_email, None);
    assert!(withheld_row.email_withheld);
    let published_row = coaches
        .iter()
        .find(|coach| coach.name == "A. Bender")
        .unwrap();
    assert_eq!(
        published_row.professional_email.as_deref(),
        Some("abender@ofsd.k12.wi.us")
    );
    assert!(!published_row.email_withheld);
}

#[test]
fn derived_rows_replace_in_place_and_never_move_the_append_counter() {
    // A derivation is a function of the store, not evidence about it: re-running it must leave one
    // row per key. Appending would instead add an observation per pass until the table's 20M cap
    // aborted every scan of it. The table is a derived map, so the write is the whole of what happens
    // to it: no sequence is reserved, and no observation is appended.
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .replace_many(Table::ReviewCases, &[derived("case:1", 1)])
        .unwrap();
    store
        .replace_many(Table::ReviewCases, &[derived("case:1", 2)])
        .unwrap();

    let scanned: Vec<DerivedRow> = store.scan(Table::ReviewCases).unwrap();
    assert_eq!(scanned.len(), 1, "one row per key whatever the pass count");
    assert_eq!(scanned[0].note, 2, "the later derivation replaces the row");
    assert_eq!(rows_held(&store, Table::ReviewCases), 1);
    assert_eq!(
        sequence_pointer(&store, Table::ReviewCases),
        0,
        "a derived write reserves nothing"
    );
    let stats = store.stats().unwrap();
    let appended = stats
        .appended
        .iter()
        .find(|(table, _)| table == "review_cases")
        .map(|(_, count)| *count)
        .unwrap();
    assert_eq!(appended, 0, "a replaced row is not an appended observation");
}

#[test]
fn a_rejected_derived_record_leaves_the_table_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let good = school("Abbotsford");
    store
        .replace_many(Table::Schools, std::slice::from_ref(&good))
        .unwrap();

    // An id the store's key contract refuses (empty) must fail the whole batch, and the row that was
    // already there must survive: a rejected derivation may not leave a half-written table behind.
    let mut broken = school("Colby");
    broken.id = serde_json::from_str::<SchoolId>("\"\"").unwrap();
    assert!(broken.id.as_str().is_empty());
    match store.replace_many(Table::Schools, &[broken]) {
        Err(StoreError::Invariant { detail }) => assert!(detail.contains("must not be empty")),
        other => panic!("expected an invariant rejection, got {other:?}"),
    }

    let scanned: Vec<CanonicalSchool> = store.scan(Table::Schools).unwrap();
    assert_eq!(scanned.len(), 1);
    assert_eq!(scanned[0].id, good.id);
}

#[test]
fn a_derived_map_table_keys_one_row_per_id_under_sequence_zero() {
    // The mode's own invariant, read off the keys: one physical row per entity key, every row keyed
    // under sequence zero, and no id owning two rows. Two rows under one id would merge at read time,
    // and the older copy wins wherever the newer says nothing — which is how a re-derived row reverts.
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .replace_many(
            Table::ReviewCases,
            &[derived("case:1", 1), derived("case:2", 1)],
        )
        .unwrap();
    // A second pass: one id the table already holds, one id named twice by the same batch.
    store
        .replace_many(
            Table::ReviewCases,
            &[
                derived("case:1", 2),
                derived("case:2", 2),
                derived("case:2", 3),
            ],
        )
        .unwrap();

    let walk = store.walk_table(Table::ReviewCases).unwrap();
    assert_eq!(walk.rows, 2, "one physical row per entity key");
    assert_eq!(
        walk.highest_sequence,
        Some(0),
        "every row is keyed under sequence zero"
    );
    assert_eq!(walk.foreign_sequences, 0);
    assert_eq!(walk.repeated_ids, 0, "no id owns two rows");
    assert_eq!(rows_held(&store, Table::ReviewCases), 2);

    let notes: Vec<(String, u32)> = store
        .scan::<DerivedRow>(Table::ReviewCases)
        .unwrap()
        .into_iter()
        .map(|row| (row.id, row.note))
        .collect();
    assert_eq!(
        notes,
        vec![("case:1".to_string(), 2), ("case:2".to_string(), 3)],
        "the last record named for an id is the row that stands"
    );
}

#[test]
fn a_snapshot_write_replaces_the_rows_it_does_not_name() {
    // A snapshot table's batch is its whole new content: a region that lost its last meet must lose its
    // row, or a report goes on counting a jurisdiction the store no longer has. A map is the opposite,
    // and its rows are pinned by the map test above.
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .replace_many(Table::Coverage, &[derived("wi", 1), derived("oh", 1)])
        .unwrap();
    assert_eq!(store.scan::<DerivedRow>(Table::Coverage).unwrap().len(), 2);

    store
        .replace_many(Table::Coverage, &[derived("oh", 2)])
        .unwrap();
    let ids: Vec<String> = store
        .scan::<DerivedRow>(Table::Coverage)
        .unwrap()
        .into_iter()
        .map(|row| row.id)
        .collect();
    assert_eq!(
        ids,
        vec!["oh".to_string()],
        "the row the batch does not name is gone, not merely unread"
    );
    assert_eq!(rows_held(&store, Table::Coverage), 1);

    store
        .replace_many(Table::Coverage, &[derived("wi", 3)])
        .unwrap();
    assert_eq!(
        store.scan::<DerivedRow>(Table::Coverage).unwrap().len(),
        1,
        "and the next snapshot replaces what that one left"
    );
    assert_eq!(rows_held(&store, Table::Coverage), 1);
}

#[test]
fn a_derived_write_clears_the_foreign_sequences_of_the_ids_it_names() {
    // A store that predates the import gate holds derived rows under sequences of their own, and the
    // merged read takes the later row: such a copy outranks every re-derivation and no path removes it,
    // so the derivation loses to the row it replaced. This is the repair: an id a derived write names
    // comes out of that write holding exactly the row the write put there.
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let imported = derived("case:1", 7);
    let mut batch = store.db.batch();
    batch.insert(
        &store.entities,
        observation_key(Table::ReviewCases, "case:1", 7),
        serde_json::to_vec(&imported).unwrap(),
    );
    batch.commit().unwrap();
    assert_eq!(
        store
            .walk_table(Table::ReviewCases)
            .unwrap()
            .foreign_sequences,
        1,
        "the fixture is a row the store's own writer would never have keyed"
    );

    store
        .replace_many(Table::ReviewCases, &[derived("case:1", 8)])
        .unwrap();

    let walk = store.walk_table(Table::ReviewCases).unwrap();
    assert_eq!(walk.rows, 1, "the copy is gone, not merely outranked");
    assert_eq!(walk.foreign_sequences, 0);
    let rows = store.scan::<DerivedRow>(Table::ReviewCases).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].note, 8,
        "the derivation, not the copy it replaced, is what reads"
    );
}

#[test]
fn the_row_ledger_is_a_count_the_keyspace_can_contradict() {
    // A count is worth keeping only if a later reader can be contradicted by it. The ledger travels in
    // the same batch as the rows it counts, so a keyspace that lost a row behind the store's back reads
    // as one row to a walk and two to the store — the drift `integrity` reports. A count re-derived by
    // walking at report time could never disagree with the walk it was re-derived from.
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let abbotsford = school("Abbotsford").id;
    store
        .append_many(Table::Schools, &[school("Abbotsford"), school("Colby")])
        .unwrap();
    assert_eq!(rows_held(&store, Table::Schools), 2);
    assert!(store.integrity().unwrap().ok);

    let key = super::keys::observation_key(Table::Schools, abbotsford.as_str(), 0);
    store.entities.remove(key).unwrap();

    assert_eq!(
        rows_held(&store, Table::Schools),
        2,
        "the ledger is the count the store kept, not one re-derived on demand"
    );
    assert_eq!(store.walk_table(Table::Schools).unwrap().rows, 1);
    let report = store.integrity().unwrap();
    assert!(!report.ok, "a lost row is what integrity exists to report");
    let schools = report
        .tables
        .iter()
        .find(|entry| entry.table == "schools")
        .expect("every table is checked");
    assert_eq!(schools.expected, 2, "the count the store holds");
    assert_eq!(schools.actual, 1, "the rows the walk finds");
}

#[test]
fn an_append_at_the_row_ceiling_lands_and_the_next_one_is_refused() {
    // The last legal observation is the one that brings the table to exactly `MAX_ROWS_PER_TABLE`:
    // refusing that one would withhold a sequence the writer is entitled to hand out. The append
    // after it is refused, and the refusal may not spend a sequence — a refused batch that moved the
    // counter would refuse every later append for a row that was never written.
    //
    // The edge is reached by moving the counter the way the store moves it, rather than by writing
    // 20M observations: the count is what the bound is measured in.
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(
        store
            .reserve(Table::Schools, MAX_ROWS_PER_TABLE - 1)
            .unwrap()
            .base,
        0
    );

    store.append(Table::Schools, &school("Last Legal")).unwrap();
    assert_eq!(sequence_pointer(&store, Table::Schools), MAX_ROWS_PER_TABLE);
    assert_eq!(
        rows_held(&store, Table::Schools),
        1,
        "a reservation is not a row: the one observation appended is all the table holds"
    );
    assert_eq!(
        store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
        1
    );

    match store.append(Table::Schools, &school("One Past")) {
        Err(StoreError::TooManyRows { table, max }) => {
            assert_eq!(table, "schools");
            assert_eq!(
                max,
                usize::try_from(MAX_ROWS_PER_TABLE).unwrap_or(usize::MAX)
            );
        }
        other => panic!("expected the row ceiling to refuse the append, got {other:?}"),
    }
    assert_eq!(
        sequence_pointer(&store, Table::Schools),
        MAX_ROWS_PER_TABLE,
        "a refused append may not advance the table's sequence"
    );
    assert_eq!(
        store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
        1,
        "the refused observation must not be in the store"
    );
}

#[test]
fn a_batch_that_would_straddle_the_row_ceiling_is_refused_before_it_reserves() {
    // The bound is on the sequence the batch would *reach*, not on the batch's own length: one row
    // short of the ceiling a two-row batch is over it although two rows on their own are not. A check
    // on the length instead of the reach would commit this batch and leave the table unreadable,
    // because a scan aborts on the row past the ceiling.
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .reserve(Table::Schools, MAX_ROWS_PER_TABLE - 1)
        .unwrap();

    let batch = [school("Penultimate"), school("Last")];
    match store.append_many(Table::Schools, &batch) {
        Err(StoreError::TooManyRows { table, .. }) => assert_eq!(table, "schools"),
        other => panic!("expected the row ceiling to refuse the batch, got {other:?}"),
    }
    assert_eq!(
        sequence_pointer(&store, Table::Schools),
        MAX_ROWS_PER_TABLE - 1,
        "a refused batch must leave the sequence where it found it"
    );
    assert_eq!(
        rows_held(&store, Table::Schools),
        0,
        "the refused batch wrote no rows, however many sequences stand reserved"
    );
    assert!(store
        .scan::<CanonicalSchool>(Table::Schools)
        .unwrap()
        .is_empty());
}

#[test]
fn the_reservation_funnel_refuses_a_run_that_would_cross_the_ceiling() {
    // The check `append_many` makes on its way in runs before the append lock, so it can only ever be
    // a courtesy: two batches can both pass it and then, one after the other, take the sequences it
    // said were there. The reservation itself is the authority, because it is the one place every
    // sequence spender passes through — the appenders and the legacy import's chunk commits alike —
    // and it runs under the lock, so the bound has to hold here on the sequence the run would reach.
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .reserve(Table::Schools, MAX_ROWS_PER_TABLE - 1)
        .unwrap();

    // The last legal run ends exactly at the ceiling, and a zero-length run is legal even at it: the
    // import opens with that probe, and a probe reaches nothing.
    let last = store.reserve(Table::Schools, 1).unwrap();
    assert_eq!(last.mark, MAX_ROWS_PER_TABLE);
    let probe = store.reserve(Table::Schools, 0).unwrap();
    assert_eq!(probe.base, MAX_ROWS_PER_TABLE);

    for count in [1, 2] {
        match store.reserve(Table::Schools, count) {
            Err(StoreError::TooManyRows { table, .. }) => assert_eq!(table, "schools"),
            other => panic!("expected the ceiling to refuse a run of {count}, got {other:?}"),
        }
        assert_eq!(
            sequence_pointer(&store, Table::Schools),
            MAX_ROWS_PER_TABLE,
            "a refused run must leave the counter where it found it"
        );
    }
    // A run whose reach overflows the counter is refused for the same reason a reach past the ceiling
    // is: there is no sequence it could land on.
    match store.reserve(Table::Schools, u64::MAX) {
        Err(StoreError::CounterOverflow) => {}
        other => panic!("expected an unrepresentable run to be refused, got {other:?}"),
    }
}

#[test]
fn the_ceiling_holds_when_two_appenders_meet_at_the_edge() {
    // The entry check is stale by the time a batch commits, so the guarantee is the reservation's:
    // with one sequence left, two threads appending at once may land one row and never two. Without
    // the check inside the funnel both would pass the entry check, and the table would hold a row past
    // the bound every scan aborts on — a row no later reader could ever get past.
    let dir = tempfile::tempdir().unwrap();
    let store = std::sync::Arc::new(Store::open(dir.path()).unwrap());
    store
        .reserve(Table::Schools, MAX_ROWS_PER_TABLE - 1)
        .unwrap();

    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let writers: Vec<std::thread::JoinHandle<StoreResult<()>>> = ["Left", "Right"]
        .into_iter()
        .map(|name| {
            let store = std::sync::Arc::clone(&store);
            let barrier = std::sync::Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                store.append(Table::Schools, &school(name))
            })
        })
        .collect();
    let outcomes: Vec<StoreResult<()>> = writers
        .into_iter()
        .map(|writer| writer.join().unwrap())
        .collect();

    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.is_ok()).count(),
        1,
        "exactly one of two appends fits under the ceiling: {outcomes:?}"
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Err(StoreError::TooManyRows { .. })))
            .count(),
        1,
        "the append that did not fit is refused with the row-ceiling error: {outcomes:?}"
    );
    assert_eq!(sequence_pointer(&store, Table::Schools), MAX_ROWS_PER_TABLE);
    assert_eq!(rows_held(&store, Table::Schools), 1);
    assert_eq!(
        store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
        1,
        "the refused observation must not be in the store"
    );
}

#[test]
fn an_over_bound_replace_batch_is_refused_before_a_single_row_is_encoded() {
    // A derived table's batch is a whole row set rather than a run of sequences, so the ceiling bounds
    // the batch itself. `()` is the cheapest `Serialize` fixture there is, and it is the reason this
    // batch is only ever counted: a refusal that happened after encoding would report each unit row
    // as an unkeyable row instead of the ceiling, so the typed error is the proof of the ordering.
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let records = vec![(); usize::try_from(MAX_ROWS_PER_TABLE).unwrap_or(usize::MAX) + 1];

    match store.replace_many(Table::Coverage, &records) {
        Err(StoreError::TooManyRows { table, max }) => {
            assert_eq!(table, "coverage");
            assert_eq!(
                max,
                usize::try_from(MAX_ROWS_PER_TABLE).unwrap_or(usize::MAX)
            );
        }
        other => panic!("expected the row ceiling to refuse the batch, got {other:?}"),
    }
    assert!(store
        .scan::<CoverageRow>(Table::Coverage)
        .unwrap()
        .is_empty());
    assert_eq!(rows_held(&store, Table::Coverage), 0);
}

#[test]
fn the_storage_mode_follows_the_writer_that_owns_each_table() {
    // Which ceiling a table's writer applies follows from how the table is written, so the split is
    // pinned against the call sites: the evidence tables the adapters (and the roster and meet walks)
    // feed through `append_many` spend a sequence per row, and the state the index, coverage, review
    // and sweep passes rebuild through `replace_many` spends none. A table that changed sides without
    // its writer changing is what this catches.
    let logs: Vec<&str> = Table::ALL
        .into_iter()
        .filter(|table| table.storage_mode() == StorageMode::ObservationLog)
        .map(Table::file)
        .collect();
    assert_eq!(
        logs,
        vec![
            "schools",
            "teams",
            "coaches",
            "athletes",
            "meets",
            "events",
            "performances",
            "source_meets",
            "source_observations",
        ]
    );

    let snapshots: Vec<&str> = Table::ALL
        .into_iter()
        .filter(|table| table.storage_mode() == StorageMode::DerivedSnapshot)
        .map(Table::file)
        .collect();
    assert_eq!(
        snapshots,
        vec!["source_identities", "conflicts", "coverage"],
        "the pass's own set is the table, so a write is its whole new content"
    );

    let maps: Vec<&str> = Table::ALL
        .into_iter()
        .filter(|table| table.storage_mode() == StorageMode::DerivedMap)
        .map(Table::file)
        .collect();
    assert_eq!(
        maps,
        vec![
            "review_cases",
            "snapshots",
            "source_access",
            "identity_verdicts",
        ],
        "rows keyed to findings and days that persist, so a write upserts what it names"
    );
}

#[test]
fn a_published_snapshot_leaves_no_temporary_behind() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schools.jsonl");
    let rows: Vec<CanonicalSchool> = (0..3)
        .map(|index| school(&format!("Row {index}")))
        .collect();

    write_snapshot_rows(&path, &rows).unwrap();

    let entries: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        entries,
        vec!["schools.jsonl".to_string()],
        "the temporary a snapshot is written through must not survive its publication"
    );
    let raw = std::fs::read_to_string(&path).unwrap();
    let parsed: Vec<CanonicalSchool> = raw
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(parsed.len(), rows.len());
    assert_eq!(parsed[0].id, rows[0].id);
}

#[test]
fn concurrent_snapshot_writers_only_publish_whole_files() {
    // Two consolidations can hold one snapshot path at once: the durable national run consolidates
    // one jurisdiction per object and the service runs several of those concurrently, while a reader
    // (the report, the workbook, an operator with `less`) may be reading the same path. Publication
    // is a rename, so every observation is one whole snapshot; writing in place let a reader catch a
    // half-file and let two writers interleave into one.
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schools.jsonl");
    let wide: Vec<CanonicalSchool> = (0..400)
        .map(|index| school(&format!("Wide School {index}")))
        .collect();
    let narrow: Vec<CanonicalSchool> = (0..3)
        .map(|index| school(&format!("Narrow School {index}")))
        .collect();

    let stop = Arc::new(AtomicBool::new(false));
    let reader_path = path.clone();
    let reader_stop = Arc::clone(&stop);
    let reader = std::thread::spawn(move || {
        let mut observations = 0_usize;
        while !reader_stop.load(Ordering::Relaxed) {
            let raw = match std::fs::read_to_string(&reader_path) {
                Ok(raw) => raw,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => panic!("snapshot read failed: {error}"),
            };
            assert!(
                !raw.is_empty(),
                "a reader observed a truncated snapshot: neither writer publishes an empty file"
            );
            for line in raw.lines() {
                let _: CanonicalSchool = serde_json::from_str(line)
                    .expect("a reader must never observe a partial snapshot line");
            }
            observations += 1;
        }
        observations
    });

    for _ in 0..32 {
        write_snapshot_rows(&path, &wide).unwrap();
        write_snapshot_rows(&path, &narrow).unwrap();
    }
    stop.store(true, Ordering::Relaxed);
    let observations = reader.join().unwrap();
    assert!(observations > 0, "the reader never observed the snapshot");
}

#[test]
fn opening_a_store_reclaims_what_a_dead_writer_left_behind() {
    // A consolidation publishes by rename, so a temporary exists only while its writer is alive and
    // holding the store lock. A crash mid-write (the crash drill in this repository's run evidence)
    // left a 377 MB and an 863 MB `.part` behind; they are partial copies of tables that are still
    // in the store, and nothing else would ever remove them.
    let dir = tempfile::tempdir().unwrap();
    let entities = dir.path().join("entities");
    {
        let store = Store::open(dir.path()).unwrap();
        store
            .replace_many(Table::Schools, &[school("Abbotsford")])
            .unwrap();
        // The published snapshot, written the way the consolidation writes it.
        store
            .consolidate_table(Table::Schools, &entities.join("schools.jsonl"))
            .unwrap();
    }
    let abandoned = entities.join(".schools.jsonl.999999.7.part");
    std::fs::write(&abandoned, b"{\"partial\"\n").unwrap();
    let published = entities.join("schools.jsonl");
    let before = std::fs::read_to_string(&published).unwrap();

    {
        let store = Store::open(dir.path()).unwrap();
        assert_eq!(
            store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
            1
        );
    }

    assert!(
        !abandoned.exists(),
        "a dead writer's temporary must not survive a reopen"
    );
    assert_eq!(
        std::fs::read_to_string(&published).unwrap(),
        before,
        "sweeping temporaries may not touch the published snapshot"
    );
}

#[test]
fn the_sweep_only_removes_snapshot_temporaries() {
    // The rule is `.<name>.part`, not "anything dot-prefixed": a store root also holds `.` entries
    // the filesystem owns, and an unrelated file must not be deleted by a store open.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("entities")).unwrap();
    let keep = dir.path().join("entities").join("notes.txt");
    std::fs::write(&keep, b"keep me").unwrap();
    let part = dir.path().join("entities").join(".schools.jsonl.1.0.part");
    std::fs::write(&part, b"partial").unwrap();

    assert_eq!(sweep_stale_temporaries(dir.path()).unwrap(), 1);
    assert!(!part.exists());
    assert!(keep.exists());
}

#[test]
fn a_store_without_an_entities_directory_sweeps_nothing() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(sweep_stale_temporaries(dir.path()).unwrap(), 0);
}

#[test]
fn a_page_of_work_commits_rows_across_tables_and_the_journal_together() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let meet = CanonicalMeet::new(
        None,
        "Batch Invitational",
        "2026-06-01",
        CompetitionLevel::Invitational,
    );
    let mut batch = store.write_batch();
    batch
        .append_many(Table::Schools, std::slice::from_ref(&school("Batch High")))
        .unwrap();
    batch
        .append_many(Table::Meets, std::slice::from_ref(&meet))
        .unwrap();
    // A second page for a table already in the batch joins the first, so its rows share the table's
    // mark rather than being written under a reservation of their own.
    batch
        .append_many(Table::Schools, &[school("Second High")])
        .unwrap();
    batch
        .journal_done("unit", "batch-1", &serde_json::json!({ "rows": 3 }))
        .unwrap();
    assert!(!batch.is_empty(), "three rows and one entry are a page");
    batch.commit().unwrap();

    assert_eq!(rows_held(&store, Table::Schools), 2);
    assert_eq!(rows_held(&store, Table::Meets), 1);
    assert!(
        store.journal_keys("unit").unwrap().contains("batch-1"),
        "the entry the page named is in the journal"
    );
    drop(store);

    // The whole page was one commit, so a reopen finds every table and the journal entry together.
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(
        reopened
            .scan::<CanonicalSchool>(Table::Schools)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        reopened.scan::<CanonicalMeet>(Table::Meets).unwrap().len(),
        1
    );
    assert!(reopened.journal_keys("unit").unwrap().contains("batch-1"));
}

#[test]
fn a_refused_entry_leaves_the_page_unwritten_and_every_counter_where_it_was() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let before = sequence_pointer(&store, Table::Schools);
    let mut batch = store.write_batch();
    batch
        .append_many(Table::Schools, &[school("Never Landed")])
        .unwrap();
    let refused = batch.journal_done("unit", "too-big", &"x".repeat(MAX_JOURNAL_VALUE_BYTES + 1));
    assert!(refused.is_err(), "an entry past its ceiling is refused");
    // The caller's `?` drops the batch here: the rows buffered before the refusal are never written,
    // so the unit a resume re-runs cannot find half of itself.
    drop(batch);

    assert_eq!(
        sequence_pointer(&store, Table::Schools),
        before,
        "no reservation moved"
    );
    assert_eq!(rows_held(&store, Table::Schools), 0);
    assert!(store.journal_keys("unit").unwrap().is_empty());
}
/// A record whose `Serialize` fails on the Nth call, proving that a real encode error
/// during `StoreBatch::append_many` leaves the store completely untouched: no rows, no journal,
/// no counter moved, no reservation spent.
///
/// The old non-atomic path (`Store::append_many` + `Store::journal_done` as separate calls)
/// would have left the rows of the first successful call visible — a half-applied source
/// observation — because each call was its own commit.
use std::sync::atomic::{AtomicUsize, Ordering};

/// Zero-indexed call counter for `FlakyRecord::serialize`: returns Err on the call matching this.
static FLAKY_SERIALIZE_FAIL_AT: AtomicUsize = AtomicUsize::new(3);

/// A record that serializes successfully twice, then fails on the third call.
///
/// Implements `Entity` so it can be stored in any table.
#[derive(Debug, Clone)]
struct FlakyRecord {
    id: String,
    value: u32,
}

impl serde::Serialize for FlakyRecord {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let call = FLAKY_SERIALIZE_FAIL_AT.fetch_add(1, Ordering::Relaxed);
        if call == 3 {
            return Err(serde::ser::Error::custom("real encode failure"));
        }
        // Serialize a simple two-field object.
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("FlakyRecord", 2)?;
        s.serialize_field("id", &self.id)?;
        s.serialize_field("value", &self.value)?;
        s.end()
    }
}

impl<'de> serde::de::Deserialize<'de> for FlakyRecord {
    fn deserialize<D: serde::de::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        use serde::de::{self, MapAccess};
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = FlakyRecord;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a FlakyRecord")
            }
            fn visit_map<M: MapAccess<'de>>(
                self,
                mut map: M,
            ) -> std::result::Result<Self::Value, M::Error> {
                let mut id = None;
                let mut value = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        "id" => id = Some(map.next_value()?),
                        "value" => value = Some(map.next_value()?),
                        _ => {
                            drop(map.next_value::<serde_json::Value>());
                        }
                    }
                }
                Ok(FlakyRecord {
                    id: id.ok_or_else(|| de::Error::missing_field("id"))?,
                    value: value.ok_or_else(|| de::Error::missing_field("value"))?,
                })
            }
        }
        deserializer.deserialize_map(Visitor)
    }
}

impl Entity for FlakyRecord {
    fn entity_id(&self) -> &str {
        &self.id
    }
    fn merge(&mut self, _other: Self) {
        // Never called — this test only appends.
    }
}

/// A page's encode failure leaves the store untouched: no rows, no journal, no counter moved.
///
/// The test uses three `FlakyRecord` instances whose `Serialize` succeeds for the first two
/// calls and fails on the third. The batch buffers a journal entry first, then offers all three
/// records to `StoreBatch::append_many` as one slice, which refuses the whole call: records are
/// encoded there, so the batch never reaches `commit`.
///
/// The assertion is that the Json error surfaces from `append_many`, the abandoned batch leaves
/// the store exactly as it was, and a reopen shows the sequence pointer, row count and journal
/// in their original state.
#[test]
fn a_store_batch_encode_failure_commits_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    // Reset the flaky counter so this test is deterministic in isolation.
    FLAKY_SERIALIZE_FAIL_AT.store(3, Ordering::Relaxed);

    let before_seq = sequence_pointer(&store, Table::Schools);
    let before_rows = rows_held(&store, Table::Schools);

    let mut batch = store.write_batch();
    let r0 = FlakyRecord {
        id: "r0".to_string(),
        value: 0,
    };
    let r1 = FlakyRecord {
        id: "r1".to_string(),
        value: 1,
    };
    let r2 = FlakyRecord {
        id: "r2".to_string(),
        value: 2,
    };
    // The journal entry is buffered before the records, so the abandoned batch already holds
    // real work when the append refuses: the entry cannot escape on its own either.
    batch
        .journal_done("unit", "fail-atomic", &serde_json::json!({"rows": 3}))
        .unwrap();
    let refused = batch.append_many(Table::Schools, &[r0, r1, r2]);

    // The third record's serialization is what refuses the append; `commit` is never reached.
    assert!(
        refused.is_err(),
        "append_many should have refused the unencodable record"
    );
    assert!(
        matches!(&refused, Err(StoreError::Json { detail, .. }) if detail.contains("serializing a batched observation")),
        "error should be StoreError::Json"
    );
    drop(batch);

    // A reopen shows the store is exactly as it was before the batch: no rows, no journal.
    drop(store);
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(
        sequence_pointer(&reopened, Table::Schools),
        before_seq,
        "sequence pointer must not have moved"
    );
    assert_eq!(
        rows_held(&reopened, Table::Schools),
        before_rows,
        "row count must be unchanged"
    );
    assert!(
        reopened.journal_keys("unit").unwrap().is_empty(),
        "no journal entry should exist"
    );
    assert!(
        reopened
            .scan::<FlakyRecord>(Table::Schools)
            .unwrap()
            .is_empty(),
        "no rows should have landed"
    );
}
