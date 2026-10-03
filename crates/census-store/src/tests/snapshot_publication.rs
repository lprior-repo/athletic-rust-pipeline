use super::*;

#[test]
fn a_published_snapshot_leaves_no_temporary_behind() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("schools.jsonl");
    let rows: Vec<CanonicalSchool> = (0..3)
        .map(|index| school(&format!("Row {index}")))
        .collect();
    write_snapshot_rows(&path, &rows)?;
    let entries: Vec<String> = std::fs::read_dir(dir.path())?
        .map(|entry| -> TestResult<_> { Ok(entry?.file_name().to_string_lossy().into_owned()) })
        .collect::<TestResult<_>>()?;
    check!(eq; entries, vec!["schools.jsonl".to_string()], "the temporary a snapshot is written through must not survive its publication");
    let raw = std::fs::read_to_string(&path)?;
    let parsed: Vec<CanonicalSchool> = raw
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    check!(eq; parsed.len(), rows.len());
    check!(eq; parsed[0].id, rows[0].id);
    Ok(())
}

#[test]
fn concurrent_snapshot_writers_only_publish_whole_files() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("schools.jsonl");
    let wide: Vec<CanonicalSchool> = (0..400)
        .map(|index| school(&format!("Wide School {index}")))
        .collect();
    let narrow: Vec<CanonicalSchool> = (0..3)
        .map(|index| school(&format!("Narrow School {index}")))
        .collect();
    write_snapshot_rows(&path, &wide)?;
    let ready = std::sync::Barrier::new(2);
    std::thread::scope(|scope| -> TestResult {
        let reader = scope.spawn(|| -> TestResult {
            ready.wait();
            for _ in 0..128 {
                let raw = std::fs::read_to_string(&path)?;
                let rows: Vec<CanonicalSchool> = raw
                    .lines()
                    .map(serde_json::from_str)
                    .collect::<Result<_, _>>()?;
                check!(
                    rows == wide || rows == narrow,
                    "reader observed an incomplete generation"
                );
            }
            Ok(())
        });
        ready.wait();
        let written = (0..32).try_for_each(|_| -> TestResult {
            write_snapshot_rows(&path, &wide)?;
            write_snapshot_rows(&path, &narrow)?;
            Ok(())
        });
        let read = reader.join().map_err(|_| "snapshot reader panicked");
        written?;
        read??;
        Ok(())
    })
}

#[test]
fn opening_a_store_reclaims_what_a_dead_writer_left_behind() -> TestResult {
    let dir = tempfile::tempdir()?;
    let entities = dir.path().join("entities");
    {
        let store = Store::open(dir.path())?;
        store.append_many(Table::Schools, &[school("Abbotsford")])?;
        store.consolidate_table(Table::Schools, &entities.join("schools.jsonl"))?;
    }
    let abandoned = entities.join(".schools.jsonl.999999.7.part");
    std::fs::write(&abandoned, b"{\"partial\"\n")?;
    let published = entities.join("schools.jsonl");
    let before = std::fs::read_to_string(&published)?;
    {
        let store = Store::open(dir.path())?;
        check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?.len(), 1);
    }
    check!(
        !abandoned.exists(),
        "a dead writer's temporary must not survive a reopen"
    );
    check!(eq; std::fs::read_to_string(&published)?, before, "sweeping temporaries may not touch the published snapshot");
    Ok(())
}

#[test]
fn the_sweep_only_removes_snapshot_temporaries() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::create_dir_all(dir.path().join("entities"))?;
    let keep = dir.path().join("entities").join("notes.txt");
    std::fs::write(&keep, b"keep me")?;
    let part = dir.path().join("entities").join(".schools.jsonl.1.0.part");
    std::fs::write(&part, b"partial")?;
    check!(eq; sweep_stale_temporaries(dir.path())?, 1);
    check!(!part.exists());
    check!(keep.exists());
    Ok(())
}

#[test]
fn a_store_without_an_entities_directory_sweeps_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    check!(eq; sweep_stale_temporaries(dir.path())?, 0);
    Ok(())
}
