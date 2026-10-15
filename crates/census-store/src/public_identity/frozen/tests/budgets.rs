use super::*;
use census_domain::public_identity_history::ReplayLimits;

#[test]
fn frozen_row_budget_precedes_operation_decode() -> TestResult {
    let (_directory, store, _writer, _) = two_births()?;
    let frozen = store.snapshot().freeze_public_identity_v1()?;
    let mut bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    bundle.operations.first_mut().ok_or("operation")?.witness = "x".repeat(65_537);
    check!(matches!(
        tamper::corrupted(&bundle)?,
        StoreError::RowTooLarge {
            bytes: 65_537,
            max: 65_536
        }
    ));
    Ok(())
}

#[test]
fn frozen_collection_decoder_refuses_one_more_than_history_limit() -> TestResult {
    let (_directory, store, _writer, _) = two_births()?;
    let frozen = store.snapshot().freeze_public_identity_v1()?;
    let mut bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    bundle.contexts = (0..=ReplayLimits::SMALL.records)
        .map(|_| wire::Context {
            root: String::new(),
            bytes: String::new(),
        })
        .collect();
    let error = tamper::corrupted(&bundle)?;
    check!(matches!(error, StoreError::Decode { key, source }
        if key == "frozen identity bundle" && source.to_string().contains("frozen identity collection exceeds bound")));
    Ok(())
}

#[test]
fn frozen_context_record_budget_is_checked_before_json_decode() -> TestResult {
    let (_directory, store, _writer, _) = two_births()?;
    let mut bytes = vec![b'x'; crate::journal::context::CONTEXT_RECORD_BYTES];
    bytes.push(b'\n');
    let result = crate::public_identity::context::from_bytes(
        &bytes,
        &digest(&bytes),
        &store.store_identity()?,
    );
    check!(matches!(result, Err(StoreError::Refused { detail })
        if detail == "invalid retained authority record bytes"));
    Ok(())
}

#[test]
fn frozen_context_record_count_is_checked_before_json_decode() -> TestResult {
    let (_directory, store, _writer, _) = two_births()?;
    let bytes = vec![b'\n'; crate::journal::context::CONTEXT_RECORDS + 1];
    let result = crate::public_identity::context::from_bytes(
        &bytes,
        &digest(&bytes),
        &store.store_identity()?,
    );
    check!(matches!(result, Err(StoreError::Refused { detail })
        if detail == "authority context exceeds retained input limits"));
    Ok(())
}
