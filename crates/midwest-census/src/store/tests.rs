use super::*;
use crate::store::read::{sweep_stale_temporaries, write_snapshot};
use census_domain::model::*;
use census_domain::UsJurisdiction;

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name)).0
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
        assert_eq!(
            store.scan::<CanonicalSchool>(Table::Schools).unwrap().len(),
            1
        );
        assert!(store
            .journal_keys("milesplit_rosters")
            .unwrap()
            .contains("wi:1"));
    }
    // The importer is idempotent: the second open sees the marker and adds nothing.
    let store = Store::open(&root).unwrap();
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
    // aborted every scan of it.
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let mut first = school("Abbotsford");
    first.city = Some("Abbotsford".into());
    store
        .replace_many(Table::Schools, &[first.clone()])
        .unwrap();
    store
        .replace_many(Table::Schools, &[first.clone()])
        .unwrap();

    let mut second = first.clone();
    second.city = Some("Colby".into());
    store.replace_many(Table::Schools, &[second]).unwrap();

    let scanned: Vec<CanonicalSchool> = store.scan(Table::Schools).unwrap();
    assert_eq!(scanned.len(), 1, "one row per key whatever the pass count");
    assert_eq!(
        scanned[0].city.as_deref(),
        Some("Colby"),
        "the later derivation replaces the row"
    );
    let stats = store.stats().unwrap();
    let schools = stats
        .tables
        .iter()
        .find(|(table, _)| table == "schools")
        .map(|(_, count)| *count)
        .unwrap();
    assert_eq!(schools, 0, "a replaced row is not an appended observation");
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
fn a_published_snapshot_leaves_no_temporary_behind() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schools.jsonl");
    let rows: Vec<CanonicalSchool> = (0..3)
        .map(|index| school(&format!("Row {index}")))
        .collect();

    write_snapshot(&path, &rows).unwrap();

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
        write_snapshot(&path, &wide).unwrap();
        write_snapshot(&path, &narrow).unwrap();
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
