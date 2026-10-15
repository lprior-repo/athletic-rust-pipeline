use super::preparation_gate;
use crate::public_identity_tests::support::{retained_state, PhysicalState, TestResult};
use crate::{GraphAuthorityReferenceV1, Store};
use serde_json::{json, Value};

const FIRST: &str = "2026-10-14T12:00:00Z";
const SECOND: &str = "2026-10-14T12:00:01Z";
const HEADER: &[u8] = b"manual-context-admission-v1\0header";

#[derive(Debug, PartialEq, Eq)]
pub(in crate::public_identity) struct PreparedBytes {
    pub record: Vec<u8>,
    pub approval: Vec<u8>,
    pub digest: String,
}

pub(in crate::public_identity) fn exercise(
    store: &Store,
    operation: &str,
    prepare: impl Fn() -> TestResult<PreparedBytes> + Sync,
) -> TestResult {
    let original = retained_state(store)?;
    let first = std::thread::scope(|scope| -> TestResult<PreparedBytes> {
        let (first_gate, first_control) = preparation_gate::pair(FIRST);
        let (second_gate, second_control) = preparation_gate::pair(SECOND);
        let action = &prepare;
        let first = scope.spawn(move || preparation_gate::with_gate(first_gate, action));
        let second = scope.spawn(move || preparation_gate::with_gate(second_gate, action));
        let graph = first_control.wait()?;
        check!(eq; second_control.wait()?, graph);
        let before = retained_state(store)?;
        unchanged_application(&original, &before)?;
        first_control.resume()?;
        let first = first
            .join()
            .map_err(|_| "first preparation thread failed")??;
        let winner = retained_state(store)?;
        once_only_preparation(&before, &winner, operation, &graph, &first)?;
        second_control.resume()?;
        let second = second
            .join()
            .map_err(|_| "second preparation thread failed")??;
        check!(eq; second, first);
        check!(eq; retained_state(store)?, winner);
        Ok(first)
    })?;
    let winner = retained_state(store)?;
    check!(eq; prepare()?, first);
    check!(eq; retained_state(store)?, winner);
    Ok(())
}

fn unchanged_application(before: &PhysicalState, after: &PhysicalState) -> TestResult {
    for name in [crate::ENTITIES, crate::META, crate::RECEIPTS] {
        check!(eq; space(after, name)?, space(before, name)?);
    }
    Ok(())
}

fn once_only_preparation(
    before: &PhysicalState,
    after: &PhysicalState,
    operation: &str,
    graph: &GraphAuthorityReferenceV1,
    prepared: &PreparedBytes,
) -> TestResult {
    unchanged_application(before, after)?;
    let mut expected = space(before, crate::JOURNAL)?.clone();
    let journal = space(after, crate::JOURNAL)?;
    let key = format!(
        "manual-context-admission-v1\0reservation\0{}",
        crate::journal::object_digest(operation.as_bytes())
    )
    .into_bytes();
    check!(!expected.iter().any(|(found, _)| found == &key));
    let raw = &journal
        .iter()
        .find(|(found, _)| found == &key)
        .ok_or("winning preparation row")?
        .1;
    let reservation: Value = serde_json::from_slice(raw)?;
    check!(eq; reservation["version"], json!(4));
    check!(eq; reservation["operation"], json!(operation));
    check!(eq; reservation["graph"], serde_json::to_value(graph)?);
    check!(eq; reservation["observed_at"], json!(FIRST));
    check!(eq; reservation["witness"]["observed_at"], json!(FIRST));
    check!(eq; reservation["witness"]["payload"]["adjudication"], serde_json::from_slice::<Value>(&prepared.record)?);
    let entry = expected
        .iter_mut()
        .find(|(found, _)| found.as_slice() == HEADER)
        .ok_or("original admission header")?;
    let mut header: Value = serde_json::from_slice(&entry.1)?;
    let charged = key
        .len()
        .checked_add(raw.len())
        .ok_or("metadata charge overflow")?;
    for (field, added) in [("bytes", u64::try_from(charged)?), ("reservations", 1)] {
        header[field] = json!(header[field]
            .as_u64()
            .ok_or("header counter")?
            .checked_add(added)
            .ok_or("header counter overflow")?);
    }
    let after_header = &journal
        .iter()
        .find(|(found, _)| found.as_slice() == HEADER)
        .ok_or("winning admission header")?
        .1;
    check!(eq; serde_json::from_slice::<Value>(after_header)?, header);
    entry.1 = after_header.clone();
    expected.push((key, raw.clone()));
    expected.sort_by(|left, right| left.0.cmp(&right.0));
    check!(eq; journal, &expected);
    Ok(())
}

fn space<'a>(state: &'a PhysicalState, name: &str) -> TestResult<&'a Vec<(Vec<u8>, Vec<u8>)>> {
    Ok(&state
        .iter()
        .find(|(found, _)| found == name)
        .ok_or("keyspace")?
        .1)
}
