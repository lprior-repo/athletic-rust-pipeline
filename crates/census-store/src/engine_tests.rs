use super::*;
use census_domain::model::*;
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name), None).0
}

fn operation() -> TestResult<EngineOperation> {
    Ok(EngineOperation::source_sweep(
        "wiaa",
        "wi",
        "2026-outdoor",
        1,
    )?)
}

fn digest() -> TestResult<EngineDigest> {
    Ok(EngineDigest::new("engine-digest-a".to_string())?)
}

fn rows(store: &Store, table: Table) -> TestResult<u64> {
    let stats = store.stats()?;
    let count = stats
        .tables
        .into_iter()
        .find(|(name, _)| name == table.file())
        .map(|(_, count)| count)
        .ok_or("stats names every table")?;
    Ok(count)
}

#[test]
fn engine_commits_observations_progress_and_receipt_atomically() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let outcome = store.engine_commit(&operation()?, &digest()?, |batch| {
        batch.append_many(Table::Schools, &[school("Abbotsford"), school("Colby")])?;
        batch.journal_done("wiaa_schools", "unit-0", &"engine-page")?;
        Ok(())
    })?;
    if !outcome.written() {
        return Err("a staged operation is written".into());
    }
    if outcome.appended() != 2 {
        return Err(format!("appended rows: {}", outcome.appended()).into());
    }
    let school_rows = rows(&store, Table::Schools)?;
    if school_rows != 2 {
        return Err(format!("school rows: {school_rows}").into());
    }
    let journal = store.journal_keys("wiaa_schools")?;
    if !journal.contains("unit-0") {
        return Err("the staged unit is recorded".into());
    }
    let receipt = store.receipt(operation()?.as_str())?;
    if receipt.is_none() {
        return Err("a committed operation leaves a receipt".into());
    }
    Ok(())
}

#[test]
fn engine_replay_of_one_operation_writes_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let stage = |batch: &mut StoreBatch| {
        batch.append_many(Table::Schools, &[school("Abbotsford")])?;
        batch.journal_done("wiaa_schools", "unit-0", &"engine-page")?;
        Ok(())
    };
    let first = store.engine_commit(&operation()?, &digest()?, stage)?;
    if !first.written() {
        return Err("the first commit writes".into());
    }
    let repeat = store.engine_commit(&operation()?, &digest()?, stage)?;
    if !repeat.repeated() {
        return Err("the replay is recognised".into());
    }
    if repeat.appended() != 0 {
        return Err(format!("appended rows: {}", repeat.appended()).into());
    }
    let school_rows = rows(&store, Table::Schools)?;
    if school_rows != 1 {
        return Err(format!("school rows: {school_rows}").into());
    }
    Ok(())
}

#[test]
fn engine_refuses_a_second_payload_under_one_operation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.engine_commit(&operation()?, &digest()?, |batch| {
        batch.append_many(Table::Schools, &[school("Abbotsford")])?;
        Ok(())
    })?;
    let other = EngineDigest::new("engine-digest-b".to_string())?;
    let refused = store.engine_commit(&operation()?, &other, |batch| {
        batch.append_many(Table::Schools, &[school("Colby")])?;
        Ok(())
    });
    if !matches!(refused, Err(StoreError::Invariant { .. })) {
        return Err("a second payload under one operation is refused".into());
    }
    let school_rows = rows(&store, Table::Schools)?;
    if school_rows != 1 {
        return Err(format!("school rows: {school_rows}").into());
    }
    Ok(())
}

#[test]
fn engine_refuses_operations_without_a_stable_scope() -> TestResult {
    if EngineOperation::new("".to_string()).is_ok() {
        return Err("an empty operation has no identity".into());
    }
    if EngineOperation::new("bare-operation".to_string()).is_ok() {
        return Err("an operation without a scope is refused".into());
    }
    if EngineOperation::jurisdiction("wi", "2026-outdoor", 0).is_ok() {
        return Err("a zero revision is refused".into());
    }
    if EngineDigest::new("".to_string()).is_ok() {
        return Err("an empty digest is refused".into());
    }
    let first = EngineOperation::jurisdiction("wi", "2026-outdoor", 1)?;
    let repeat = EngineOperation::jurisdiction("wi", "2026-outdoor", 1)?;
    if first != repeat {
        return Err("the same scope mints the same operation".into());
    }
    if first.as_str() != "jurisdiction:wi:2026-outdoor:1" {
        return Err(format!("operation identity: {}", first.as_str()).into());
    }
    Ok(())
}

#[test]
fn engine_staging_failure_writes_neither_rows_nor_receipts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let refused: StoreResult<Application> = store.engine_commit(&operation()?, &digest()?, |_| {
        Err(StoreError::Refused {
            detail: "staged page failed validation".to_string(),
        })
    });
    if !matches!(refused, Err(StoreError::Refused { .. })) {
        return Err("a refused stage writes nothing".into());
    }
    let school_rows = rows(&store, Table::Schools)?;
    if school_rows != 0 {
        return Err(format!("school rows: {school_rows}").into());
    }
    let receipt = store.receipt(operation()?.as_str())?;
    if receipt.is_some() {
        return Err("a failed stage leaves no receipt".into());
    }
    Ok(())
}
