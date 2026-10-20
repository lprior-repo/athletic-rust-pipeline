use super::*;

fn read_held(
    store: &Store,
    raw: &[u8],
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<crate::lineage::RetainedIdentity> {
    store
        .journal
        .insert(
            Store::journal_key("export-lineage-v1", "store-identity"),
            raw,
        )
        .map_err(|source| StoreError::Write { source })?;
    crate::lineage::read_admitted(
        &store.db.snapshot(),
        &store.journal,
        &mut crate::acquisition::admission::ReadMeter::Recovery(budget),
    )
}
fn one_reservation(raw: &[u8]) -> StoreResult<u64> {
    let mut reads = AcquisitionReadBudget::for_selection();
    codec::reserve_decode(raw, &mut reads)?;
    Ok(reads.recovery_totals().allocations.owned.used)
}
fn hostile() -> Vec<u8> {
    format!(
        "{{\"key\":\"store-identity\",\"payload\":{}0{}}}",
        "{\"x\":".repeat(112),
        "}".repeat(112)
    )
    .into_bytes()
}

#[test]
fn lineage_two_value_passes_each_debit_before_parse_and_never_reset() -> TestResult {
    let (_dir, store, _, _) = fixture_for(CaptureProtocolVersion::V2)?;
    let raw = hostile();
    let one = one_reservation(&raw)?;
    let twice = one.checked_mul(2).ok_or("reservation")?;
    let mut reads = AcquisitionReadBudget::for_selection();
    reads.recovery_limit_owned(twice);
    check!(matches!(
        read_held(&store, &raw, &mut reads),
        Err(StoreError::StoreLineage(
            crate::lineage::StoreLineageError::MalformedEnvelope
        ))
    ));
    check!(eq; reads.recovery_totals().allocations.owned.used, twice);
    check!(eq; reads.recovery_totals().allocations.calls.used, 2);
    check!(matches!(
        crate::lineage::read(&store.db.snapshot(), &store.journal),
        Err(StoreError::StoreLineage(
            crate::lineage::StoreLineageError::MalformedEnvelope
        ))
    ));
    check!(matches!(
        read_held(&store, &raw, &mut reads),
        Err(StoreError::Capture(CaptureError::ReadBudgetLimitExceeded {
            resource: "allocation",
            ..
        }))
    ));
    check!(eq; reads.recovery_totals().allocations.owned.used, twice);
    let mut refused = AcquisitionReadBudget::for_selection();
    refused.recovery_limit_owned(twice.checked_sub(1).ok_or("N-1")?);
    check!(matches!(
        read_held(&store, &raw, &mut refused),
        Err(StoreError::Capture(CaptureError::ReadBudgetLimitExceeded {
            resource: "allocation",
            ..
        }))
    ));
    check!(eq; refused.recovery_totals().allocations.owned.used, one);
    check!(eq; refused.recovery_totals().allocations.calls.used, 1);
    Ok(())
}

#[test]
fn lineage_first_syntax_failure_never_requests_a_second_decode() -> TestResult {
    let (_dir, store, _, _) = fixture_for(CaptureProtocolVersion::V2)?;
    let raw = b"{";
    let mut reads = AcquisitionReadBudget::for_selection();
    match read_held(&store, raw, &mut reads) {
        Err(StoreError::Decode { key, source }) => {
            check!(eq; key, "export-lineage-v1/store-identity");
            check!(eq; source.classify(), serde_json::error::Category::Eof);
            check!(eq; source.to_string(), "EOF while parsing an object at line 1 column 1");
        }
        _ => return Err("lineage syntax error changed".into()),
    }
    check!(eq; reads.recovery_totals().allocations.owned.used, one_reservation(raw)?);
    check!(eq; reads.recovery_totals().allocations.calls.used, 1);
    Ok(())
}

#[test]
fn lineage_valid_identity_preserves_exact_legacy_outcome_under_two_reservations() -> TestResult {
    let (_dir, store, writer, _) = fixture_for(CaptureProtocolVersion::V2)?;
    let raw = format!(
        "{{\"key\":\"store-identity\",\"payload\":\"{}\"}}",
        writer.store_identity().as_str()
    )
    .into_bytes();
    let mut reads = AcquisitionReadBudget::for_selection();
    let expected = writer.store_identity();
    check!(matches!(read_held(&store, &raw, &mut reads),
        Ok(crate::lineage::RetainedIdentity::Present(identity)) if &identity == expected));
    check!(
        matches!(crate::lineage::read(&store.db.snapshot(), &store.journal),
        Ok(crate::lineage::RetainedIdentity::Present(identity)) if &identity == expected)
    );
    check!(eq; reads.recovery_totals().allocations.owned.used, one_reservation(&raw)?.checked_mul(2).ok_or("twice")?);
    Ok(())
}

#[test]
fn pinned_two_pass_nested_object_leaf_requests_exceed_old_single_reservation() -> TestResult {
    check!(eq; std::mem::size_of::<String>(), 24);
    check!(eq; std::mem::size_of::<serde_json::Value>(), 32);
    let fields = std::mem::size_of::<usize>()
        .checked_add(4)
        .and_then(|header| {
            std::mem::size_of::<String>()
                .checked_add(std::mem::size_of::<serde_json::Value>())
                .and_then(|pair| pair.checked_mul(11))
                .and_then(|cells| header.checked_add(cells))
        })
        .ok_or("leaf fields")?;
    let leaf_lower = fields
        .checked_add(7)
        .map(|bytes| bytes / 8 * 8)
        .ok_or("leaf alignment")?;
    let request_lower = leaf_lower
        .checked_mul(225)
        .ok_or("two payloads plus first envelope")?;
    check!(u64::try_from(request_lower)? > one_reservation(&hostile())?);
    Ok(())
}
