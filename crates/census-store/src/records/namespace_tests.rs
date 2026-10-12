use super::*;

#[test]
fn durable_record_objects_preserve_store_integrity() -> TestResult {
    let (_dir, store) = store()?;
    completed(&store)?;
    let report = store.integrity()?;
    check!(eq; report.ok, true);
    check!(eq; report.corruptions.len(), 0);
    Ok(())
}
#[test]
fn completion_receipt_uses_the_authoritative_receipt_codec() -> TestResult {
    let (_dir, store) = store()?;
    completed(&store)?;
    let receipt = store
        .receipt(&format!("records/unit/{}", id('c')))?
        .ok_or("missing receipt")?;
    let unit = store.get_unit_record(&id('c'))?.ok_or("missing")?;
    let UnitState::Complete { output } = unit.state() else {
        return Err("unit not complete".into());
    };
    check!(eq; receipt.digest, output.as_str());
    check!(eq; receipt.appended, 0);
    Ok(())
}
