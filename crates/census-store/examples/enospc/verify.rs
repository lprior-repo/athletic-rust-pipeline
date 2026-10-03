use super::{records, Acknowledged, ExampleResult, Failed, BATCH_COUNT, MAX_ATTEMPTS, MAX_RECORDS};
use census_domain::model::CanonicalSchool;
use census_store::{Store, StoreError, StoreResult, Table};

fn invariant(detail: &str) -> StoreError {
    StoreError::Invariant {
        detail: detail.to_string(),
    }
}

fn expected_records(
    acknowledged: &[Acknowledged],
    failed: &Failed,
) -> StoreResult<[bool; MAX_RECORDS]> {
    if acknowledged.is_empty() || acknowledged.len() > MAX_ATTEMPTS.saturating_add(2) {
        return Err(invariant(
            "acknowledgements exceed the probe operation budget",
        ));
    }
    let failed_end = failed
        .start
        .checked_add(BATCH_COUNT)
        .ok_or(StoreError::CounterOverflow)?;
    if failed_end > super::RECOVERY_START {
        return Err(invariant("failed batch is outside the drain budget"));
    }
    let mut expected = [false; MAX_RECORDS];
    acknowledged.iter().try_for_each(|ack| -> StoreResult<()> {
        let end = ack
            .start
            .checked_add(ack.count)
            .ok_or(StoreError::CounterOverflow)?;
        if ack.count == 0 || ack.count > BATCH_COUNT || end > MAX_RECORDS {
            return Err(invariant("acknowledged batch exceeds the record budget"));
        }
        (ack.start..end).try_for_each(|index| {
            if (failed.start..failed_end).contains(&index) {
                return Err(invariant("acknowledged and failed batches overlap"));
            }
            let slot = expected.get_mut(index).ok_or(StoreError::CounterOverflow)?;
            if *slot {
                return Err(invariant("acknowledged batches overlap"));
            }
            *slot = true;
            Ok(())
        })
    })?;
    Ok(expected)
}

fn exact_records(
    store: &Store,
    expected: &[bool; MAX_RECORDS],
    failed: &Failed,
) -> StoreResult<()> {
    let total = u64::try_from(expected.iter().filter(|present| **present).count())
        .map_err(|_| StoreError::CounterOverflow)?;
    let walk = store.walk_table(Table::Schools)?;
    if walk.rows != total || walk.repeated_ids != 0 {
        return Err(invariant(
            "physical school observations are missing, unexpected or duplicated",
        ));
    }
    let failed_end = failed
        .start
        .checked_add(BATCH_COUNT)
        .ok_or(StoreError::CounterOverflow)?;
    let mut seen = [false; MAX_RECORDS];
    store
        .snapshot()
        .for_each_observation(Table::Schools, |row: CanonicalSchool| {
            let index = row
                .name
                .strip_prefix("School_")
                .and_then(|index| index.parse::<usize>().ok())
                .ok_or_else(|| invariant("unexpected school name in cold readback"))?;
            if (failed.start..failed_end).contains(&index) {
                return Err(invariant("failed atomic batch leaked a school observation"));
            }
            if expected.get(index) != Some(&true) {
                return Err(invariant("unexpected school observation in cold readback"));
            }
            if row != records::school(index)? {
                return Err(invariant(
                    "school identity or full supporting payload changed",
                ));
            }
            let slot = seen.get_mut(index).ok_or(StoreError::CounterOverflow)?;
            if *slot {
                return Err(invariant("school observation was appended more than once"));
            }
            *slot = true;
            Ok(())
        })?;
    if &seen != expected {
        return Err(invariant(
            "acknowledged exact school observations are missing",
        ));
    }
    Ok(())
}

fn exact_receipts(
    store: &Store,
    acknowledged: &[Acknowledged],
    failed: &Failed,
) -> StoreResult<()> {
    let count = u64::try_from(acknowledged.len()).map_err(|_| StoreError::CounterOverflow)?;
    if store.receipt_count()? != count {
        return Err(invariant("cold readback receipt count changed"));
    }
    if store.receipt(&failed.operation)?.is_some() {
        return Err(invariant("failed atomic batch leaked a receipt"));
    }
    acknowledged.iter().try_for_each(|ack| {
        let digest = records::digest(&records::schools(ack.start, ack.count)?)?;
        let count = u64::try_from(ack.count).map_err(|_| StoreError::CounterOverflow)?;
        if ack.receipt.operation.is_empty()
            || ack.receipt.digest != digest
            || ack.receipt.appended != count
            || ack.receipt.at.is_empty()
            || store.receipt(&ack.receipt.operation)?.as_ref() != Some(&ack.receipt)
        {
            return Err(invariant(
                "acknowledged exact receipt or payload digest changed",
            ));
        }
        Ok(())
    })
}

pub(super) fn state(
    store: &Store,
    acknowledged: &[Acknowledged],
    failed: &Failed,
) -> ExampleResult<()> {
    let expected = expected_records(acknowledged, failed)?;
    exact_records(store, &expected, failed)?;
    exact_receipts(store, acknowledged, failed)?;
    Table::ALL
        .into_iter()
        .filter(|table| *table != Table::Schools)
        .try_for_each(|table| {
            if store.walk_table(table)?.rows != 0 {
                return Err(invariant(
                    "probe has unexpected observations outside the school table",
                ));
            }
            Ok(())
        })?;
    Ok(())
}
