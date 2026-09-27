use super::*;

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

    store.append(Table::Schools, &first).unwrap();
    store
        .meta
        .insert("import_offset:schools", committed.to_string().as_bytes())
        .unwrap();

    store.import_legacy().unwrap();

    let rows = store.scan::<CanonicalSchool>(Table::Schools).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(store.stats().unwrap().observations, 2);
    assert!(store.meta.contains_key("imported:schools").unwrap());
}

#[test]
fn legacy_lines_are_trimmed_before_they_are_parsed() {
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
fn bulk_legacy_import_counts_every_committed_chunk() -> Result<(), Box<dyn std::error::Error>> {
    for imported_rows in [20_000_u64, 20_001_u64] {
        let dir = tempfile::tempdir()?;
        let entities = dir.path().join("entities");
        std::fs::create_dir_all(&entities)?;
        let row = school("Imported school");
        let body = serde_json::to_vec(&row)?;
        let mut output =
            std::io::BufWriter::new(std::fs::File::create(entities.join("schools.jsonl"))?);
        for _ in 0..imported_rows {
            std::io::Write::write_all(&mut output, &body)?;
            std::io::Write::write_all(&mut output, b"\n")?;
        }
        std::io::Write::flush(&mut output)?;
        drop(output);

        let existing = school("Previously acquired school");
        let existing_body = serde_json::to_vec(&existing)?;
        let store = Store::open(dir.path())?;
        store.append(Table::Schools, &existing)?;
        assert_eq!(store.import_legacy()?.observations, imported_rows);
        drop(store);

        let reopened = Store::open(dir.path())?;
        let expected = imported_rows.checked_add(1).ok_or("row count overflow")?;
        assert_eq!(reopened.count(Table::Schools)?, expected);
        let existing_key = observation_key(Table::Schools, existing.entity_id(), 0);
        let retained = reopened
            .entities
            .get(existing_key)?
            .ok_or("missing existing row")?;
        assert_eq!(retained.as_ref(), existing_body.as_slice());
        for sequence in 1..=imported_rows {
            let key = observation_key(Table::Schools, row.entity_id(), sequence);
            let retained = reopened.entities.get(key)?.ok_or("missing imported row")?;
            assert_eq!(retained.as_ref(), body.as_slice());
        }
        assert_eq!(reopened.import_legacy()?.observations, 0);
        assert_eq!(reopened.count(Table::Schools)?, expected);
    }
    Ok(())
}
