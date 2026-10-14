use super::*;

#[test]
fn historical_terminal_unknown_v2_loss_keeps_exact_decode_boundary() -> TestResult {
    let (_dir, store, writer, owner) = fixture()?;
    let occurrence = retained(&store, &writer, &owner, 200)?;
    let exact = work(&owner)?;
    let held = registered(&store, &writer, &owner)?;
    let grant =
        writer.continuation_for_owned_meet(&store, &owner, UsJurisdiction::Michigan, "123")?;
    let key = wrapper_key("terminal", &exact, occurrence.attempt())?;
    let physical = Store::journal_key(SOURCE_ACQUISITION_PHASE, &key);
    let raw = store.journal.get(&physical)?.ok_or("terminal")?;
    let text = std::str::from_utf8(&raw)?;
    let old = "\"kind\":\"response_retained\"";
    let new = "\"kind\":\"prepared_retention_failed\"";
    let position = text.find(old).ok_or("terminal kind")?;
    let changed = text.replacen(old, new, 1);
    store.journal.insert(&physical, changed.as_bytes())?;
    let before = rows(&store)?;
    let column = position.checked_add(new.len()).ok_or("column")?;
    let expected = format!("unknown variant `prepared_retention_failed`, expected one of `response_retained`, `validated_not_modified`, `not_dispatched`, `failed_uncaptured`, `outcome_unknown` at line 1 column {column}");
    exact_decode(
        read::read_work(
            &store.db.snapshot(),
            &store.journal,
            &store.receipts,
            &exact.id()?,
            &mut AcquisitionReadBudget::for_work(),
            CaptureProtocolVersion::V1,
        ),
        &key,
        &expected,
    )?;
    exact_decode(
        store.inspect_acquisition_work(&held, &mut AcquisitionReadBudget::for_work()),
        &key,
        &expected,
    )?;
    exact_decode(
        store.register_acquisition_work(&owner, exact.clone(), grant),
        &key,
        &expected,
    )?;
    exact_decode(
        crate::acquisition::qualification::validate_format(
            store.root(),
            &store.db.snapshot(),
            &store.meta,
            &store.journal,
            &store.receipts,
            &store.store_format()?,
        ),
        &key,
        &expected,
    )?;
    check!(eq; rows(&store)?, before);
    Ok(())
}

#[test]
fn all_historical_wrappers_keep_exact_version_decode_errors_in_actual_readers() -> TestResult {
    for suffix in [
        "binding", "reserved", "armed", "response", "prepared", "terminal", "result",
    ] {
        for version in [2, 3] {
            let (_dir, store, writer, owner) = fixture()?;
            let occurrence = retained(&store, &writer, &owner, 200)?;
            let exact = work(&owner)?;
            let held = registered(&store, &writer, &owner)?;
            let grant = writer.continuation_for_owned_meet(
                &store,
                &owner,
                UsJurisdiction::Michigan,
                "123",
            )?;
            let key = wrapper_key(suffix, &exact, occurrence.attempt())?;
            let physical = Store::journal_key(SOURCE_ACQUISITION_PHASE, &key);
            let raw = store.journal.get(&physical)?.ok_or("wrapper")?;
            let text = std::str::from_utf8(&raw)?;
            let needle = "\"payload\":{\"version\":1";
            let position = text.find(needle).ok_or("ordered wrapper")?;
            let changed = text.replacen(needle, &format!("\"payload\":{{\"version\":{version}"), 1);
            store.journal.insert(&physical, changed.as_bytes())?;
            let before = rows(&store)?;
            let column = position.checked_add(needle.len()).ok_or("column")?;
            let expected = format!("unsupported capture protocol at line 1 column {column}");
            exact_decode(
                read::read_work(
                    &store.db.snapshot(),
                    &store.journal,
                    &store.receipts,
                    &exact.id()?,
                    &mut AcquisitionReadBudget::for_work(),
                    CaptureProtocolVersion::V1,
                ),
                &key,
                &expected,
            )?;
            exact_decode(
                store.inspect_acquisition_work(&held, &mut AcquisitionReadBudget::for_work()),
                &key,
                &expected,
            )?;
            exact_decode(
                store.register_acquisition_work(&owner, exact.clone(), grant),
                &key,
                &expected,
            )?;
            exact_decode(
                crate::acquisition::qualification::validate_format(
                    store.root(),
                    &store.db.snapshot(),
                    &store.meta,
                    &store.journal,
                    &store.receipts,
                    &store.store_format()?,
                ),
                &key,
                &expected,
            )?;
            check!(eq; rows(&store)?, before);
        }
    }
    Ok(())
}
fn wrapper_key(
    suffix: &str,
    work: &AcquisitionWorkV1,
    attempt: &AcquisitionAttemptId,
) -> TestResult<String> {
    Ok(match suffix {
        "binding" => format!("slot/{}/binding", work.slot().id()?),
        "reserved" | "prepared" | "terminal" => {
            format!("{}/{suffix}", read::attempt_prefix(attempt))
        }
        "result" => format!("work/{}/result", work.id()?),
        _ => format!("{}/dispatch/0/{suffix}", read::attempt_prefix(attempt)),
    })
}
fn exact_decode<T>(result: StoreResult<T>, expected_key: &str, expected: &str) -> TestResult {
    match result {
        Err(StoreError::Decode { key, source }) => {
            check!(eq; key, expected_key);
            check!(eq; source.to_string(), expected);
            Ok(())
        }
        _ => Err("strict historical decoder error changed".into()),
    }
}
