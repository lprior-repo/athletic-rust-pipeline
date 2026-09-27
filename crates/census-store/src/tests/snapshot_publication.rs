use super::*;

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
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schools.jsonl");
    let wide: Vec<CanonicalSchool> = (0..400)
        .map(|index| school(&format!("Wide School {index}")))
        .collect();
    let narrow: Vec<CanonicalSchool> = (0..3)
        .map(|index| school(&format!("Narrow School {index}")))
        .collect();

    write_snapshot_rows(&path, &wide).unwrap();
    let ready = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        let reader = scope.spawn(|| {
            ready.wait();
            for _ in 0..128 {
                let raw = std::fs::read_to_string(&path).unwrap();
                let rows: Vec<CanonicalSchool> = raw
                    .lines()
                    .map(|line| serde_json::from_str(line).unwrap())
                    .collect();
                assert!(
                    rows == wide || rows == narrow,
                    "reader observed an incomplete generation"
                );
            }
        });
        ready.wait();
        for _ in 0..32 {
            write_snapshot_rows(&path, &wide).unwrap();
            write_snapshot_rows(&path, &narrow).unwrap();
        }
        reader.join().unwrap();
    });
}

#[test]
fn opening_a_store_reclaims_what_a_dead_writer_left_behind() {
    let dir = tempfile::tempdir().unwrap();
    let entities = dir.path().join("entities");
    {
        let store = Store::open(dir.path()).unwrap();
        store
            .append_many(Table::Schools, &[school("Abbotsford")])
            .unwrap();
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
