use super::*;

#[test]
fn metadata_row_size_refusal_precedes_non_utf8_key_refusal_without_enrollment_writes() -> TestResult
{
    let root = tempfile::tempdir()?;
    let config = prepare(root.path())?;
    let bytes = crate::MAX_INSTALLATION_RECORD_BYTES
        .checked_add(1)
        .ok_or("row length")?;
    let store = Store::open_existing(root.path())?;
    store.meta.insert([0xff], vec![b'x'; bytes])?;
    store.flush()?;
    drop(store);
    let mut control = Store::installation_control_existing(root.path())?;
    let before = witness(&control.store)?;
    match control.enroll(&config) {
        Err(StoreError::RowTooLarge { bytes: actual, max }) => {
            equal(actual, bytes)?;
            equal(max, crate::MAX_INSTALLATION_RECORD_BYTES)?;
        }
        other => return Err(format!("oversized non-UTF8 metadata={other:?}").into()),
    }
    equal(witness(&control.store)?, before)
}

#[test]
fn bounded_non_utf8_metadata_retains_historical_metadata_refusal_without_enrollment_writes(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let config = prepare(root.path())?;
    let store = Store::open_existing(root.path())?;
    store.meta.insert([0xff], b"x")?;
    store.flush()?;
    drop(store);
    let mut control = Store::installation_control_existing(root.path())?;
    let before = witness(&control.store)?;
    match control.enroll(&config) {
        Err(StoreError::Installation(InstallationError::HistoricalStatePresent { kind })) => {
            equal(kind, HistoricalStateKind::Metadata)?;
        }
        other => return Err(format!("non-UTF8 metadata={other:?}").into()),
    }
    equal(witness(&control.store)?, before)
}

#[test]
fn missing_table_counter_refusal_precedes_oversized_metadata_scan_without_enrollment_writes(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let config = prepare(root.path())?;
    let bytes = crate::MAX_INSTALLATION_RECORD_BYTES
        .checked_add(1)
        .ok_or("row length")?;
    let store = Store::open_existing(root.path())?;
    store
        .meta
        .remove(format!("rows:{}", Table::Schools.file()))?;
    store.meta.insert([0xff], vec![b'x'; bytes])?;
    store.flush()?;
    drop(store);
    let mut control = Store::installation_control_existing(root.path())?;
    let before = witness(&control.store)?;
    match control.enroll(&config) {
        Err(StoreError::Installation(InstallationError::HistoricalStatePresent { kind })) => {
            equal(kind, HistoricalStateKind::Metadata)?;
        }
        other => return Err(format!("missing counter and oversized metadata={other:?}").into()),
    }
    equal(witness(&control.store)?, before)
}
