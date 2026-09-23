//! Tests for the one-time legacy import and for the writes it shares with a crash.
//!
//! The import is explicit, so a test lays the pre-Fjall journal out on disk and then calls
//! [`Store::import_legacy`] on the open store: an import that succeeds proves the migration finished,
//! and one that fails proves it refused — with the file and the line in the error.
//! `an_open_is_a_read_and_the_import_is_explicit` holds that apart from opening the store. The `meta`
//! rows a *failing* import must not write are read straight from the Fjall database, because a refused
//! import leaves no `Store` to read them through; that is also the only way to build the state no
//! writer will produce, such as a table one row short of the ceiling.
//!
//! Every fixture is a small file, and every assertion is on what a caller of the store can observe:
//! the rows a scan returns, the counts a table reports, the markers `meta` holds, and the error an
//! open failed with.

use super::legacy::{read_legacy_line, MAX_LEGACY_LINE_BYTES};
use super::read::directory_bytes;
use super::sequences::mark_key;
use super::*;
use census_domain::model::*;
use census_domain::UsJurisdiction;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::Path;

/// One school's serialized observation, exactly as the pre-Fjall journals hold one.
fn observation(name: &str) -> String {
    let school = CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name)).0;
    serde_json::to_string(&school).unwrap()
}

/// An item of the pre-Fjall resume ledger: one unit of work a run finished.
fn journal_entry(key: &str) -> String {
    format!("{{\"key\":\"{key}\",\"at\":\"2026-09-20\",\"payload\":{{\"athletes\":5}}}}\n")
}

/// One conflict row as the derivation pass writes it: the id is a function of the finding, so the same
/// finding re-derives the same id.
fn conflict(subject: &str, detail: &str) -> String {
    serde_json::to_string(&RetainedConflict::new(
        "duplicate",
        subject,
        subject,
        detail,
    ))
    .unwrap()
}

/// A store root that has been opened once, so its Fjall database and keyspaces exist and a test can
/// lay out the fixture the next import reads — or the `meta` row that describes a table too large to
/// write.
fn opened_root() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    drop(Store::open(dir.path()).unwrap());
    dir
}

/// Open `root` and run the one-time import the way a migrating path does.
///
/// [`Store::open`] does not import — opening is a read — so a fixture that expects imported rows says
/// when the migration happened. `an_open_is_a_read_and_the_import_is_explicit` holds the two apart.
fn imported(root: &Path) -> StoreResult<Store> {
    let store = Store::open(root)?;
    store.import_legacy()?;
    Ok(store)
}

/// Run `f` against a store's keyspaces with no `Store` open: Fjall holds the database exclusively,
/// so what a failing import left behind — and the state no writer can produce — is read and written
/// this way.
fn with_keyspaces<T>(root: &Path, f: impl FnOnce(&fjall::Keyspace, &fjall::Keyspace) -> T) -> T {
    let db = fjall::Database::builder(root.join(DB_DIR)).open().unwrap();
    let meta = db
        .keyspace(META, fjall::KeyspaceCreateOptions::default)
        .unwrap();
    let journal = db
        .keyspace(JOURNAL, fjall::KeyspaceCreateOptions::default)
        .unwrap();
    let out = f(&meta, &journal);
    db.persist(fjall::PersistMode::SyncAll).unwrap();
    out
}

/// A `meta` row as an open store holds it.
fn meta_row(store: &Store, key: &str) -> Option<Vec<u8>> {
    store.meta.get(key).unwrap().map(|value| value.to_vec())
}

/// A `meta` row as it survives a refused import.
fn unopened_meta_row(root: &Path, key: &str) -> Option<Vec<u8>> {
    with_keyspaces(root, |meta, _| {
        meta.get(key).unwrap().map(|value| value.to_vec())
    })
}

/// How many entries a phase's ledger holds in the store, counted without a `Store` at all.
fn unopened_journal_entries(root: &Path, phase: &str) -> usize {
    with_keyspaces(root, |_, journal| {
        journal.prefix(Store::journal_key(phase, "")).count()
    })
}

/// The error an explicit import of `root` refused with.
///
/// The store is dropped before the caller reads the `meta` rows a refused import must not have
/// written: Fjall owns the database exclusively, so those rows are read straight from it, and a
/// refused import leaves no `Store` to read them through.
fn import_failure(root: &Path) -> StoreError {
    let store = Store::open(root).expect("opening a store is a read, so no journal can refuse it");
    match store.import_legacy() {
        Ok(report) => panic!("the import should have refused this store: {report:?}"),
        Err(error) => error,
    }
}

/// The `detail` of a legacy import failure, and the assertion that the failure is one.
fn legacy_detail(error: &StoreError) -> &str {
    match error {
        StoreError::Legacy { detail } => detail,
        other => panic!("expected a legacy import failure, got {other}"),
    }
}

/// Whether this process runs as root, in which case the kernel ignores the permission bits a test
/// needs to make an entry unreadable.
fn running_as_root() -> bool {
    std::fs::metadata("/proc/self")
        .map(|metadata| metadata.uid() == 0)
        .unwrap_or(false)
}

// -- the entity journals (items 10, 11, 12) -------------------------------------------------------

#[test]
fn an_open_is_a_read_and_the_import_is_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("entities")).unwrap();
    std::fs::create_dir_all(root.join("journal")).unwrap();

    // A root holding both pre-Fjall journals. The open that a store *read* performs — a status verb, an
    // integrity check, a console session — used to import them, so measuring the store rewrote it, and
    // an operator asking a store how big it was could move two million rows as the answer.
    let observations = format!("{}\n{}\n", observation("Abbotsford"), observation("Colby"));
    let journal_bytes = u64::try_from(observations.len()).unwrap();
    std::fs::write(root.join("entities/schools.jsonl"), &observations).unwrap();
    std::fs::write(
        root.join("journal/milesplit_rosters.jsonl"),
        journal_entry("wi:1"),
    )
    .unwrap();

    let store = Store::open(root).unwrap();
    // Opening a store holds nothing of the journals: no rows, no count, no ledger, no marker, no
    // offset. Every one of these is what an open-time import would have written.
    assert_eq!(
        store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
        0,
        "an open imports nothing"
    );
    assert_eq!(store.count(Table::Schools).unwrap(), 0);
    assert!(store.journal_keys("milesplit_rosters").unwrap().is_empty());
    assert!(meta_row(&store, "imported:schools").is_none());
    assert!(meta_row(&store, "import_offset:schools").is_none());
    assert!(meta_row(&store, "imported:resume-journals").is_none());
    assert_eq!(
        std::fs::read_to_string(root.join("entities/schools.jsonl")).unwrap(),
        observations,
        "the journal is the record of what the database was built from, not something an open consumes"
    );

    // The migrating path says so, and then everything the open did not do is there: the rows, the
    // count, the offset that follows them, both markers, and the resume ledger.
    let report = store.import_legacy().unwrap();
    assert_eq!(report.observations, 2);
    assert_eq!(report.skipped, 0);
    assert_eq!(
        store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
        2
    );
    assert_eq!(store.count(Table::Schools).unwrap(), 2);
    assert_eq!(
        meta_row(&store, "import_offset:schools"),
        Some(journal_bytes.to_string().into_bytes())
    );
    assert!(meta_row(&store, "imported:schools").is_some());
    assert!(meta_row(&store, "imported:resume-journals").is_some());
    assert!(store
        .journal_keys("milesplit_rosters")
        .unwrap()
        .contains("wi:1"));
    drop(store);

    // The store a later open sees holds the import, and that open adds nothing to it: a read stays a
    // read whether or not there is work on disk for it to refuse to do.
    let store = Store::open(root).unwrap();
    assert_eq!(
        store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
        2,
        "a second open imports nothing on top of the explicit import"
    );
    assert_eq!(store.count(Table::Schools).unwrap(), 2);
}

#[test]
fn a_crash_truncated_tail_stops_the_import_at_the_last_complete_line() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("entities")).unwrap();

    // What a kill in the middle of a write leaves: two complete observations and the head of a third,
    // with no newline. The importer used to parse that head as a row, so every later open failed on
    // the same bytes and the table could never be imported at all.
    let mut journal = format!("{}\n{}\n", observation("Abbotsford"), observation("Colby"));
    let committed = u64::try_from(journal.len()).unwrap();
    journal.push_str("{\"id\":\"ott\",\"name\":\"Ott");
    assert!(u64::try_from(journal.len()).unwrap() > committed);
    std::fs::write(root.join("entities/schools.jsonl"), &journal).unwrap();

    let store = imported(root).unwrap();
    assert_eq!(
        store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
        2,
        "the truncated tail is not a row"
    );
    assert_eq!(
        meta_row(&store, "import_offset:schools"),
        Some(committed.to_string().into_bytes()),
        "the committed offset is the line boundary before the tail, never past it"
    );
    assert!(
        meta_row(&store, "imported:schools").is_some(),
        "the durable prefix was imported in full, so the table is imported once"
    );
    drop(store);

    // A later import succeeds — the wedge is gone — and it does not import the prefix a second time.
    let store = imported(root).unwrap();
    assert_eq!(
        store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
        2
    );
    assert_eq!(store.count(Table::Schools).unwrap(), 2);
}

#[test]
fn an_interior_malformed_line_fails_the_import_and_marks_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("entities")).unwrap();

    // A complete line that is not an observation sits between two that are. An unterminated tail is
    // the one shape a crash explains; this one the file's own writer produced, so it is corruption.
    let journal = format!(
        "{}\n{{\"id\":\"broken\",\"n\n{}\n",
        observation("Abbotsford"),
        observation("Colby")
    );
    std::fs::write(root.join("entities/schools.jsonl"), &journal).unwrap();

    let error = import_failure(root);
    let detail = legacy_detail(&error);
    assert!(detail.contains("schools.jsonl"), "{detail}");
    assert!(detail.contains("line 2"), "{detail}");

    // Nothing is recorded as imported and no offset was committed: the file is still the store's
    // import source, not a journal a later open skips.
    assert!(unopened_meta_row(root, "imported:schools").is_none());
    assert!(unopened_meta_row(root, "import_offset:schools").is_none());
    assert_eq!(unopened_journal_entries(root, "schools"), 0);

    // With the offending line repaired the same file imports, from its head — the refusal left no
    // half-imported state that a later open would mistake for progress.
    std::fs::write(
        root.join("entities/schools.jsonl"),
        format!("{}\n{}\n", observation("Abbotsford"), observation("Colby")),
    )
    .unwrap();
    let store = imported(root).unwrap();
    assert_eq!(
        store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
        2
    );
}

#[test]
fn the_row_ceiling_counts_the_rows_a_table_already_holds() {
    let dir = opened_root();
    let root = dir.path();

    // A store whose schools table already holds one observation fewer than the ceiling. The mark is
    // the row count its next open seeds that table from, so this is the state a table that size opens
    // in; writing twenty million rows to prove it would be twenty million rows of test. The import
    // marker is cleared so the next import reads the journal again.
    with_keyspaces(root, |meta, _| {
        meta.insert(
            mark_key(Table::Schools),
            (MAX_ROWS_PER_TABLE - 1).to_string().as_bytes(),
        )
        .unwrap();
        meta.remove("imported:schools").unwrap();
    });
    std::fs::create_dir_all(root.join("entities")).unwrap();
    std::fs::write(
        root.join("entities/schools.jsonl"),
        format!(
            "{}\n{}\n{}\n",
            observation("Abbotsford"),
            observation("Colby"),
            observation("Dorchester")
        ),
    )
    .unwrap();

    // The ceiling is the table's, so exactly one row fits: the journal's second row is the one that
    // does not, and the import says which line that is. A ceiling applied to the chunk instead —
    // which is what a counter reset every twenty thousand rows did — lets all three in.
    let error = import_failure(root);
    let detail = legacy_detail(&error);
    assert!(detail.contains("line 2"), "{detail}");
    assert!(detail.contains(&MAX_ROWS_PER_TABLE.to_string()), "{detail}");
    assert!(
        unopened_meta_row(root, "imported:schools").is_none(),
        "a refused import may not mark its table imported"
    );
    assert!(unopened_meta_row(root, "import_offset:schools").is_none());
    assert_eq!(
        unopened_journal_entries(root, "schools"),
        0,
        "the single row that fit is not committed either: the refusal aborts the import"
    );
}

#[test]
fn a_line_past_the_ceiling_is_refused_without_being_read_whole() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("entities")).unwrap();

    // One observation, then a line bigger than any observation this store will hold. The line is
    // refused on its size — parsing it as a row would make all of it resident first, and the chunk's
    // byte budget is only consulted once a line exists.
    let mut journal = format!("{}\n", observation("Abbotsford"));
    journal.push_str(&"x".repeat(MAX_LEGACY_LINE_BYTES + 1));
    journal.push('\n');
    std::fs::write(root.join("entities/schools.jsonl"), &journal).unwrap();

    let error = import_failure(root);
    let detail = legacy_detail(&error);
    assert!(detail.contains("line 2"), "{detail}");
    assert!(
        detail.contains(&MAX_LEGACY_LINE_BYTES.to_string()),
        "{detail}"
    );
    assert!(unopened_meta_row(root, "imported:schools").is_none());
}

#[test]
fn the_reader_never_holds_more_than_the_line_ceiling() {
    // Four times the ceiling, handed over as one buffer by the `Cursor`: the reader must refuse
    // without retaining what it refuses. `read_until` cannot — it grows the caller's buffer until the
    // newline arrives, however far away that is, which is exactly how a multi-gigabyte line became
    // resident memory.
    let mut input = vec![b'x'; MAX_LEGACY_LINE_BYTES.saturating_mul(4)];
    input.push(b'\n');
    let mut reader = std::io::Cursor::new(input);
    let mut line = Vec::new();

    let error = read_legacy_line(&mut reader, &mut line, Path::new("schools.jsonl"), 2)
        .expect_err("a line past the ceiling is refused");
    assert!(matches!(error, StoreError::Legacy { .. }), "{error}");
    assert!(
        line.len() <= MAX_LEGACY_LINE_BYTES,
        "the reader held {} bytes of a line past the {MAX_LEGACY_LINE_BYTES} byte ceiling",
        line.len()
    );
}

#[test]
fn an_over_long_unterminated_tail_is_refused_rather_than_read_to_its_end() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("entities")).unwrap();

    // The reader refuses at the ceiling without knowing whether the line ever ends: reading on to
    // find out is unbounded work on a file already known to be malformed. So an unterminated tail
    // larger than any observation is corruption, not the crash tail a smaller one is.
    let mut journal = format!("{}\n", observation("Abbotsford"));
    journal.push_str(&"y".repeat(MAX_LEGACY_LINE_BYTES + 1));
    std::fs::write(root.join("entities/schools.jsonl"), &journal).unwrap();

    let error = import_failure(root);
    let detail = legacy_detail(&error);
    assert!(detail.contains("line 2"), "{detail}");
    assert!(
        detail.contains(&MAX_LEGACY_LINE_BYTES.to_string()),
        "{detail}"
    );
}

// -- the resume ledger (item 13) ------------------------------------------------------------------

#[test]
fn a_malformed_resume_journal_entry_fails_the_import() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("journal")).unwrap();

    // Two finished units of work and, between them, a line that is not an entry. The ledger is what
    // stops finished work being done twice, so an entry the import dropped would be work nothing
    // records as done — the import refuses instead, and says which line to repair.
    let journal = format!(
        "{}{}{}",
        journal_entry("wi:1"),
        "{\"key\":\"wi:2\",\"at\":\"2026-09-20\",\"payload\"\n",
        journal_entry("wi:3")
    );
    std::fs::write(root.join("journal/milesplit_rosters.jsonl"), &journal).unwrap();

    let error = import_failure(root);
    let detail = legacy_detail(&error);
    assert!(detail.contains("milesplit_rosters.jsonl"), "{detail}");
    assert!(detail.contains("line 2"), "{detail}");

    assert!(
        unopened_meta_row(root, "imported:resume-journals").is_none(),
        "a refused import may not mark the migration complete"
    );
    assert_eq!(
        unopened_journal_entries(root, "milesplit_rosters"),
        0,
        "a refused import commits no entries, so the next run reads the whole ledger"
    );
}

#[test]
fn a_resume_journal_line_without_a_key_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("journal")).unwrap();

    // Valid JSON that is not an entry: it names no unit of work, so importing it would record
    // nothing while marking the ledger done.
    let journal = format!(
        "{}{}",
        journal_entry("wi:1"),
        "{\"at\":\"2026-09-20\",\"payload\":{\"athletes\":5}}\n"
    );
    std::fs::write(root.join("journal/milesplit_rosters.jsonl"), &journal).unwrap();

    let error = import_failure(root);
    let detail = legacy_detail(&error);
    assert!(detail.contains("line 2"), "{detail}");
    assert!(detail.contains("key"), "{detail}");
}

#[test]
fn a_truncated_resume_journal_tail_imports_the_entries_before_it() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("journal")).unwrap();

    // The one line a crash can leave unfinished stops the import where it stands: the entries before
    // it are the ledger, the head of the one after it is not an entry, and the migration is complete
    // for what the file durably held.
    let journal = format!(
        "{}{}",
        journal_entry("wi:1"),
        "{\"key\":\"wi:2\",\"at\":\"2026-09-"
    );
    std::fs::write(root.join("journal/milesplit_rosters.jsonl"), &journal).unwrap();

    let store = imported(root).unwrap();
    let keys = store.journal_keys("milesplit_rosters").unwrap();
    assert_eq!(keys.len(), 1, "{keys:?}");
    assert!(keys.contains("wi:1"));
    assert_eq!(
        store.journal_payloads("milesplit_rosters").unwrap().len(),
        1
    );
    assert!(meta_row(&store, "imported:resume-journals").is_some());
}

// -- derived tables (a legacy file for a rebuilt table) -------------------------------------------

#[test]
fn a_derived_tables_legacy_journal_is_left_alone() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("entities")).unwrap();

    // A pre-Fjall store's tree: an append-only table's journal, which is evidence, beside a derived
    // table's, which is a read model the derivation pass rebuilds from the merged rows.
    std::fs::write(
        root.join("entities/schools.jsonl"),
        format!("{}\n{}\n", observation("Abbotsford"), observation("Colby")),
    )
    .unwrap();
    let derived_path = root.join("entities/conflicts.jsonl");
    std::fs::write(
        &derived_path,
        format!(
            "{}\n{}\n",
            conflict("abbotsford", "stale detail"),
            conflict("colby", "another stale row")
        ),
    )
    .unwrap();

    let store = imported(root).unwrap();
    assert_eq!(
        store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
        2,
        "an append-only table's journal is still imported"
    );
    // The derived table holds nothing. Imported rows would be keyed with the sequences the import is
    // spending, while the pass keys its rows under sequence zero — and a read merges the later key of
    // an id over the earlier one, so an imported copy would win over the row the pass wrote.
    assert_eq!(
        store.walk_table(Table::Conflicts).unwrap().rows,
        0,
        "a derived table's legacy rows are not imported"
    );
    assert!(store
        .scan::<RetainedConflict>(Table::Conflicts)
        .unwrap()
        .is_empty());
    // Both markers: the one-time import is finished with the table, and the skip says which file it
    // left alone. The file itself is still there, because a store has to open.
    assert!(meta_row(&store, "imported:conflicts").is_some());
    assert_eq!(
        meta_row(&store, "skipped:conflicts"),
        Some(derived_path.display().to_string().into_bytes())
    );
    assert!(derived_path.exists());

    // Re-running the import reports what it did with the journals it found: the observations it
    // imported, and the derived journal it refused. The markers are what make a later import a no-op,
    // so clearing them — including the offset, or the re-run would resume at the end of the file it
    // already read — is what re-runs it.
    store.meta.remove("imported:schools").unwrap();
    store.meta.remove("import_offset:schools").unwrap();
    store.meta.remove("imported:conflicts").unwrap();
    let report = store.import_legacy().unwrap();
    assert_eq!(report.observations, 2);
    assert_eq!(report.skipped, 1);
}

#[test]
fn a_derivation_pass_after_a_skip_holds_only_the_derived_rows() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("entities")).unwrap();

    // A stale read model whose second row carries the id the next derivation writes: the row an import
    // would leave behind, and the row the pass would then fail to displace.
    let subject = "abbotsford-co-op";
    std::fs::write(
        root.join("entities/conflicts.jsonl"),
        format!(
            "{}\n{}\n",
            conflict("colby", "another stale row"),
            conflict(subject, "stale detail from the pre-Fjall store")
        ),
    )
    .unwrap();

    let store = imported(root).unwrap();
    assert_eq!(
        store.walk_table(Table::Conflicts).unwrap().rows,
        0,
        "the skip leaves the table for the derivation pass to fill"
    );

    // The pass's own write for a derived table — `index::derive`, which replaces the queues it derived
    // — keys every row under sequence zero.
    let fresh = RetainedConflict::new("duplicate", subject, subject, "detail the pass derived");
    store
        .replace_many(Table::Conflicts, std::slice::from_ref(&fresh))
        .unwrap();

    let rows = store.scan::<RetainedConflict>(Table::Conflicts).unwrap();
    assert_eq!(
        rows,
        vec![fresh],
        "no imported row survives the merge over the derived one"
    );
    // And the shape a derived table has to keep: one physical row per id, every row keyed under
    // sequence zero. A row keyed above zero is the copy a read merges over the row that should have
    // replaced it, which is what the store's own walk reports as a foreign sequence.
    let walk = store.walk_table(Table::Conflicts).unwrap();
    assert_eq!(walk.rows, 1);
    assert_eq!(walk.foreign_sequences, 0);
    assert_eq!(walk.repeated_ids, 0);
}

// -- the journal reads (item 14) ------------------------------------------------------------------

#[test]
fn a_journal_key_that_is_not_utf8_is_corruption() {
    let dir = opened_root();
    let root = dir.path();
    with_keyspaces(root, |_, journal| {
        // `<phase>\0` followed by bytes no key of this store can hold: the shape a flipped bit or a
        // foreign writer leaves.
        let mut key = Store::journal_key("milesplit_rosters", "");
        key.extend_from_slice(&[0xff, 0xfe]);
        journal
            .insert(
                key,
                b"{\"key\":\"wi:1\",\"at\":\"2026-09-20\",\"payload\":null}".as_slice(),
            )
            .unwrap();
    });

    let store = Store::open(root).unwrap();
    // Reading it as "not there" would hand a resumed run a shorter ledger than the store holds, and
    // the run would repeat work the store already recorded as finished.
    let error = store.journal_keys("milesplit_rosters").unwrap_err();
    assert!(matches!(error, StoreError::Invariant { .. }), "{error}");
    assert_eq!(
        store.journal_payloads("milesplit_rosters").unwrap().len(),
        1,
        "the entry is readable; it is its key that is not"
    );
}

#[test]
fn a_journal_entry_without_a_payload_is_corruption() {
    let dir = opened_root();
    let root = dir.path();
    {
        let store = Store::open(root).unwrap();
        store
            .journal_done(
                "milesplit_rosters",
                "wi:1",
                &serde_json::json!({ "athletes": 5 }),
            )
            .unwrap();
    }
    with_keyspaces(root, |_, journal| {
        journal
            .insert(
                Store::journal_key("milesplit_rosters", "wi:2"),
                b"{\"key\":\"wi:2\",\"at\":\"2026-09-20\"}".as_slice(),
            )
            .unwrap();
    });

    let store = Store::open(root).unwrap();
    let keys = store.journal_keys("milesplit_rosters").unwrap();
    assert!(keys.contains("wi:1") && keys.contains("wi:2"), "{keys:?}");
    // The key is there and the payload is not: a report rebuilt from these entries would be rebuilt
    // from less than the store holds, so the read refuses instead of leaving the entry out.
    let error = store.journal_payloads("milesplit_rosters").unwrap_err();
    assert!(matches!(error, StoreError::Invariant { .. }), "{error}");
}

// -- the store's own size (item 23) ---------------------------------------------------------------

#[test]
fn stats_refuse_a_root_they_cannot_read() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    // A store root that is gone is not a store of zero bytes: `stats` reports that it could not
    // measure the store rather than reporting an empty one.
    std::fs::remove_dir_all(dir.path()).unwrap();
    let error = store
        .stats()
        .expect_err("an unreadable root is not zero bytes");
    assert!(matches!(error, StoreError::Io { .. }), "{error}");
}

#[test]
fn directory_bytes_refuses_a_path_that_is_not_a_directory() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("not-a-directory");
    std::fs::write(&file, b"x").unwrap();

    let error = directory_bytes(&file).expect_err("a path that cannot be listed is not zero bytes");
    assert!(matches!(error, StoreError::Io { .. }), "{error}");
}

#[test]
fn directory_bytes_refuses_a_subtree_whose_entries_it_cannot_stat() {
    let dir = tempfile::tempdir().unwrap();
    let blocked = dir.path().join("blocked");
    std::fs::create_dir(&blocked).unwrap();
    std::fs::write(blocked.join("payload"), b"x").unwrap();
    if running_as_root() {
        // Permission bits do not bind root, so there is no unreadable subtree to build here.
        return;
    }

    // Readable but not searchable: the walk can list this directory and cannot measure what it
    // holds. It used to fold the whole subtree in as zero bytes, so a store holding 500 GB behind
    // one such directory reported as an empty one.
    let mut permissions = std::fs::metadata(&blocked).unwrap().permissions();
    permissions.set_mode(0o400);
    std::fs::set_permissions(&blocked, permissions).unwrap();

    let error = directory_bytes(dir.path()).expect_err("an unmeasurable subtree is not zero bytes");
    assert!(matches!(error, StoreError::Io { .. }), "{error}");

    // Leave the temporary directory removable.
    let mut permissions = std::fs::metadata(&blocked).unwrap().permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(&blocked, permissions).unwrap();
}

/// The streaming merged scan: that it yields exactly what the collecting scan yields, that a refused
/// visitor stops it, and that a table's rows are never held at once.
///
/// The last one is the measurement, and it is why the module lives here: the property it holds is the
/// reason `for_each_merged` exists, and a number printed by a passing test is the only form of it a
/// later reader can check.
mod merged_scan {
    use super::*;

    /// The process's resident set in KiB, read from `/proc/self/status`.
    ///
    /// The measurement below is taken from this rather than from an instrumented allocator because the
    /// crate is `#![forbid(unsafe_code)]` and a counting `GlobalAlloc` is `unsafe impl` by definition.
    #[cfg(target_os = "linux")]
    fn resident_kib() -> u64 {
        let status =
            std::fs::read_to_string("/proc/self/status").expect("the process status is readable");
        status
            .lines()
            .find_map(|line| line.strip_prefix("VmRSS:"))
            .and_then(|value| value.trim().strip_suffix("kB"))
            .and_then(|value| value.trim().parse::<u64>().ok())
            .expect("VmRSS is reported in KiB")
    }

    /// A legacy journal holding `lines`, one observation per line.
    fn journal(lines: &[String]) -> String {
        let mut out = String::new();
        for line in lines {
            out.push_str(line);
            out.push('\n');
        }
        out
    }

    /// Store `lines` as the pre-Fjall `schools` journal and import them, so a table holds those rows.
    fn store_of(root: &Path, lines: &[String]) -> Store {
        std::fs::create_dir_all(root.join("entities")).unwrap();
        std::fs::write(root.join("entities/schools.jsonl"), journal(lines)).unwrap();
        imported(root).unwrap()
    }

    #[test]
    fn the_streamed_pass_yields_what_the_collecting_pass_yields() {
        let dir = tempfile::tempdir().unwrap();
        let first = observation("Abbotsford");
        // A second version of the same id: the name decides the id, so a changed field is what makes
        // this a merge rather than a duplicate. The assertion is the fixture's contract with the test.
        let second = first.replace("\"WI\"", "\"MN\"");
        assert_ne!(
            second, first,
            "the fixture needs two different versions of one id"
        );
        let lines = vec![
            first.clone(),
            second,
            observation("Colby"),
            observation("Durand"),
            observation("Durand"),
        ];
        let store = store_of(dir.path(), &lines);

        let collected: Vec<serde_json::Value> = store
            .scan::<CanonicalSchool>(Table::Schools)
            .unwrap()
            .iter()
            .map(|row| serde_json::to_value(row).unwrap())
            .collect();
        let mut streamed = Vec::new();
        let visited = store
            .for_each_merged::<CanonicalSchool>(Table::Schools, |row| {
                streamed.push(serde_json::to_value(&row).unwrap());
                Ok(())
            })
            .unwrap();

        assert_eq!(visited as usize, collected.len());
        assert_eq!(
            streamed, collected,
            "the two passes yield the same rows in the same order"
        );
        let ids: Vec<&str> = collected
            .iter()
            .map(|row| row["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids.len(), 3, "{ids:?}: five lines are three ids");
        assert!(
            ids.windows(2).all(|pair| pair[0] < pair[1]),
            "one row per id, in id order: {ids:?}"
        );
    }

    #[test]
    fn a_refused_visit_stops_the_merged_pass() {
        let dir = tempfile::tempdir().unwrap();
        let lines = vec![
            observation("Abbotsford"),
            observation("Colby"),
            observation("Durand"),
        ];
        let store = store_of(dir.path(), &lines);

        let mut visited = 0_u64;
        let refused = store
            .for_each_merged::<CanonicalSchool>(Table::Schools, |_| {
                visited = visited.saturating_add(1);
                Err(StoreError::Invariant {
                    detail: "the snapshot could not write this row".to_string(),
                })
            })
            .expect_err("a refused visit is the pass's own refusal");
        assert!(
            matches!(refused, StoreError::Invariant { .. }),
            "{refused:?}"
        );
        assert_eq!(
            visited, 1,
            "the pass stops at the refusal instead of merging the rest"
        );
    }

    #[test]
    fn a_streamed_consolidation_writes_the_merged_table() {
        let dir = tempfile::tempdir().unwrap();
        let lines = vec![
            observation("Abbotsford"),
            observation("Colby"),
            observation("Colby"),
            observation("Durand"),
        ];
        let store = store_of(dir.path(), &lines);
        let out = dir.path().join("out/schools.jsonl");

        let written = store
            .consolidate::<CanonicalSchool>(Table::Schools, &out)
            .unwrap();
        assert_eq!(written.rows, 3, "four lines are three ids");
        assert_eq!(written.withheld, 0, "no school row withholds a mailbox");

        // The published file is the JSONL the whole table would have produced, in id order: the
        // streamed writer splits the file into rows, not the merge into a different table.
        let published: Vec<serde_json::Value> = std::fs::read_to_string(&out)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let collected: Vec<serde_json::Value> = store
            .scan::<CanonicalSchool>(Table::Schools)
            .unwrap()
            .iter()
            .map(|row| serde_json::to_value(row).unwrap())
            .collect();
        assert_eq!(published, collected);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn a_streamed_pass_holds_one_id_where_the_collected_pass_holds_the_table() {
        let dir = tempfile::tempdir().unwrap();
        let mut lines = Vec::with_capacity(50_000);
        for index in 0..50_000_u32 {
            lines.push(observation(&format!("school {index:05}")));
        }
        let store = store_of(dir.path(), &lines);
        drop(lines);
        assert_eq!(store.count(Table::Schools).unwrap(), 50_000);

        // Warm the store with a pass whose rows are thrown away: both measured passes then read the
        // same warm cache, so what they cost differs by the rows they hold and not by first touch.
        let mut warm = 0_u64;
        store
            .for_each_merged::<CanonicalSchool>(Table::Schools, |_| {
                warm = warm.saturating_add(1);
                Ok(())
            })
            .unwrap();
        assert_eq!(warm, 50_000);

        let baseline = resident_kib();
        let mut streamed = 0_u64;
        let mut stream_peak = baseline;
        store
            .for_each_merged::<CanonicalSchool>(Table::Schools, |row| {
                // The row is touched so the visit cannot be optimised away, and then dropped: the
                // only entity alive between visits is the one being merged.
                std::hint::black_box(row.id.as_str().len());
                streamed = streamed.saturating_add(1);
                stream_peak = stream_peak.max(resident_kib());
                Ok(())
            })
            .unwrap();
        let collected = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
        let collected_peak = resident_kib();
        let collected_rows = collected.len();
        drop(collected);

        let stream_growth = stream_peak.saturating_sub(baseline);
        let collected_growth = collected_peak.saturating_sub(baseline);
        println!(
            "50k ids: streamed {streamed} rows rss {baseline} -> {stream_peak} KiB (+{stream_growth});              collected {collected_rows} rows rss -> {collected_peak} KiB (+{collected_growth})"
        );
        assert_eq!(streamed as usize, collected_rows);
        assert!(
            stream_growth.saturating_mul(4) < collected_growth,
            "streaming grew rss by {stream_growth} KiB against {collected_growth} KiB for              collecting the same table: the stream is not holding one id"
        );
    }
}
