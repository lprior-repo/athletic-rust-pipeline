use super::*;

pub(super) fn all(store: &Store) -> TestResult<Vec<Value>> {
    Ok(store.journal_payloads(WITNESS_PHASE)?)
}

pub(super) fn single(store: &Store, target: &SchoolId, mutation: &str) -> TestResult<Value> {
    let values = all(store)?;
    check!(eq; values.len(), 1);
    let witness = values
        .into_iter()
        .next()
        .ok_or("application witness missing")?;
    check!(eq; witness.get("application_version").and_then(Value::as_str), Some(APPLICATION_VERSION));
    check!(eq; witness.get("target").and_then(Value::as_str), Some(target.as_str()));
    check!(eq; witness.get("mutation").and_then(Value::as_str), Some(mutation));
    let provenance = witness
        .get("provenance")
        .ok_or("application has no provenance")?;
    check!(eq; witness.get("scope_digest").and_then(Value::as_str), Some(serialized_digest(provenance)?.as_str()));
    let digest = serialized_digest(&witness)?;
    let operation = format!("{OPERATION_PREFIX}{digest}");
    let receipt = store
        .receipt(&operation)?
        .ok_or("witness has no atomic receipt")?;
    check!(eq; receipt.digest, digest);
    check!(eq; receipt.credited, false);
    check!(eq; store.journal_keys(WITNESS_PHASE)?.len(), 1);
    Ok(witness)
}

pub(super) fn effect<'a>(witness: &'a Value, table: Table) -> TestResult<&'a Value> {
    witness
        .get("effects")
        .and_then(Value::as_array)
        .ok_or("application has no effect bindings")?
        .iter()
        .find(|effect| effect.get("table").and_then(Value::as_str) == Some(table.file()))
        .ok_or_else(|| "application lacks requested effect binding".into())
}

pub(super) fn assert_no_application(store: &Store) -> TestResult {
    check!(eq; store.receipt_count()?, 0);
    check!(eq; store.journal_keys(WITNESS_PHASE)?.len(), 0);
    Ok(())
}
