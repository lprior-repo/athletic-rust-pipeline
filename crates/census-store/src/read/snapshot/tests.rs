use super::{read_rows, sweep_stale_temporaries};
use crate::StoreError;
use census_domain::model::{normalize_name, CanonicalSchool};
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name), None).0
}

fn row(name: &str) -> TestResult<String> {
    Ok(serde_json::to_string(&school(name))?)
}

fn stage_snapshot(dir: &tempfile::TempDir, lines: &[String]) -> TestResult<std::path::PathBuf> {
    let path = dir.path().join("schools.jsonl");
    let mut body = lines.join("\n");
    body.push('\n');
    std::fs::write(&path, body)?;
    Ok(path)
}

#[test]
fn a_malformed_middle_row_fails_the_read_and_names_its_line() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = stage_snapshot(
        &dir,
        &[
            row("Head School")?,
            "{\"id\":\"broken\"".to_string(),
            row("Tail School")?,
        ],
    )?;
    match read_rows::<CanonicalSchool>(&path) {
        Err(StoreError::SnapshotRow {
            path: named,
            line,
            source: _,
        }) => {
            check!(eq; named, path);
            check!(eq; line, 2, "the malformed row is the second line of the file");
        }
        Err(error) => return Err(error.into()),
        Ok(_) => return Err("a malformed middle row must fail the read".into()),
    }
    Ok(())
}

#[test]
fn a_truncated_tail_row_fails_the_read_instead_of_vanishing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = stage_snapshot(&dir, &[row("Head School")?, "{\"id\":\"trunc".to_string()])?;
    match read_rows::<CanonicalSchool>(&path) {
        Err(StoreError::SnapshotRow { line, .. }) => {
            check!(eq; line, 2, "the malformed row is the last line of the file")
        }
        Err(error) => return Err(error.into()),
        Ok(_) => return Err("a truncated tail row must fail the read".into()),
    }
    Ok(())
}

#[test]
fn a_whole_snapshot_reads_every_row() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = stage_snapshot(
        &dir,
        &[row("School 0")?, row("School 1")?, row("School 2")?],
    )?;
    let read = read_rows::<CanonicalSchool>(&path)?;
    let names: Vec<&str> = read.iter().map(|school| school.name.as_str()).collect();
    check!(eq; names, ["School 0", "School 1", "School 2"]);
    Ok(())
}

#[test]
fn an_absent_snapshot_reads_as_an_empty_table() -> TestResult {
    let dir = tempfile::tempdir()?;
    let read = read_rows::<CanonicalSchool>(&dir.path().join("schools.jsonl"))?;
    check!(read.is_empty());
    Ok(())
}

#[test]
fn the_sweep_reclaims_a_temporary_in_out_as_well_as_in_entities() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = crate::Store::open(dir.path())?;
    let entities = dir.path().join("entities");
    std::fs::create_dir_all(&entities)?;
    let out = dir.path().join("out");
    std::fs::create_dir_all(&out)?;
    let staged = [
        entities.join(".schools.jsonl.4294967295.0.part"),
        out.join(".best-results-co2027.jsonl.4294967295.1.part"),
    ];
    for path in &staged {
        std::fs::write(path, b"half a file")?;
    }
    let keep = out.join("best-results-co2027.jsonl");
    std::fs::write(&keep, b"{}\n")?;
    check!(eq; sweep_stale_temporaries(dir.path(), &store.meta)?, 2);
    check!(
        staged.iter().all(|path| !path.exists()),
        "both temporaries are reclaimed"
    );
    check!(keep.exists(), "the published sidecar is untouched");
    Ok(())
}

#[test]
fn reclamation_retains_live_owner_scratch_and_foreign_part_files() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = crate::Store::open(dir.path())?;
    let out = dir.path().join("out");
    let live = out.join(format!(".schools.jsonl.{}.0.part", std::process::id()));
    let foreign = out.join(".foreign.part");
    std::fs::write(&live, b"paused live writer")?;
    std::fs::write(&foreign, b"not store-owned scratch")?;
    check!(eq; sweep_stale_temporaries(dir.path(), &store.meta)?, 0);
    check!(eq; std::fs::read(live)?, b"paused live writer");
    check!(eq; std::fs::read(foreign)?, b"not store-owned scratch");
    Ok(())
}

#[test]
fn owner_dead_scratch_reclamation_resumes_in_bounded_byte_slices() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = crate::Store::open(dir.path())?;
    let paths: Vec<_> = (0..3)
        .map(|index| {
            store
                .out_dir()
                .join(format!(".rows.jsonl.4294967295.{index}.part"))
        })
        .collect();
    for path in &paths {
        std::fs::File::create(path)?.set_len(8 * 1024 * 1024)?;
    }
    check!(eq; store.reclaim_owner_dead_scratch()?, 2);
    check!(eq; paths.iter().filter(|path| path.exists()).count(), 1);
    check!(eq; store.reclaim_owner_dead_scratch()?, 1);
    check!(paths.iter().all(|path| !path.exists()));
    check!(eq; store.reclaim_owner_dead_scratch()?, 0);
    Ok(())
}
#[test]
fn sweep_reclaims_oversized_abandoned_temporary_larger_than_sixteen_mib() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = crate::Store::open(dir.path())?;
    let oversized = store.out_dir().join(".huge.jsonl.4294967295.0.part");
    std::fs::File::create(&oversized)?.set_len(20 * 1024 * 1024)?;
    check!(oversized.exists());
    let live = store
        .out_dir()
        .join(format!(".keep.jsonl.{}.0.part", std::process::id()));
    std::fs::write(&live, b"live owner")?;
    check!(eq; store.reclaim_owner_dead_scratch()?, 1);
    check!(!oversized.exists(), "oversized temporary is reclaimed");
    check!(live.exists(), "live owner temporary is untouched");
    Ok(())
}

#[test]
fn verify_layout_reports_an_empty_store_as_uninspected() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = crate::Store::open(dir.path())?;
    check!(
        !store.snapshot().verify_layout()?,
        "an empty store holds no writer layout to inspect"
    );
    Ok(())
}

#[test]
fn verify_layout_accepts_a_real_writer_layout() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = crate::Store::open(dir.path())?;
    let schools = [school("Alpha"), school("Beta")];
    store.append_many(crate::Table::Schools, &schools)?;
    check!(store.snapshot().verify_layout()?);
    Ok(())
}

#[test]
fn verify_layout_rejects_non_ascii_observation_id() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = crate::Store::open(dir.path())?;
    let mut key = crate::keys::observation_prefix(crate::Table::Schools);
    key.extend_from_slice(b"bad-\xFF");
    key.push(0);
    key.extend_from_slice(&0_u64.to_be_bytes());
    store
        .entities
        .insert(key, serde_json::to_vec(&school("Bad"))?)?;
    let result = store.snapshot().verify_layout();
    match result {
        Err(crate::StoreError::Invariant { detail }) => {
            check!(
                detail.contains("non-ascii observation id"),
                "non-ascii id must be rejected: {detail}"
            );
        }
        other => return Err(format!("expected invariant, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn verify_layout_covers_generation_partitioned_rows() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = crate::Store::open(dir.path())?;
    let key = crate::keys::derived_key(crate::Table::Conflicts, 1, b"conflict-1");
    store.entities.insert(
        key.clone(),
        serde_json::to_vec(&serde_json::json!({"id": "conflict-1"}))?,
    )?;
    check!(store.snapshot().verify_layout()?);
    store
        .entities
        .insert(key, serde_json::to_vec(&serde_json::json!({"id": ""}))?)?;
    let result = store.snapshot().verify_layout();
    match result {
        Err(crate::StoreError::Invariant { detail }) => {
            check!(
                detail.contains("observation id must not be empty"),
                "a derived row with an empty id must be rejected: {detail}"
            );
        }
        other => return Err(format!("expected invariant, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn verify_layout_rejects_corrupt_entity_value() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = crate::Store::open(dir.path())?;
    let key = crate::keys::observation_key(crate::Table::Schools, "bad-value", 0);
    store.entities.insert(key, b"not json")?;
    let result = store.snapshot().verify_layout();
    match result {
        Err(crate::StoreError::Json { .. }) => {}
        other => return Err(format!("expected json error, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn verify_layout_rejects_malformed_observation_id_in_value() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = crate::Store::open(dir.path())?;
    let key = crate::keys::observation_key(crate::Table::Schools, "good-key", 0);
    let bad_value = serde_json::json!({"id": ""});
    store
        .entities
        .insert(key, serde_json::to_vec(&bad_value)?)?;
    let result = store.snapshot().verify_layout();
    match result {
        Err(crate::StoreError::Invariant { detail }) => {
            check!(
                detail.contains("observation id must not be empty"),
                "empty id in value must be rejected: {detail}"
            );
        }
        other => return Err(format!("expected invariant, got {other:?}").into()),
    }
    Ok(())
}
