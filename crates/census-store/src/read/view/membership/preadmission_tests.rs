use super::*;
use crate::{PhysicalReadAdmissionV1, StoreError};
use serde_json::json;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const PHASE: &str = "namespace_preadmission_fixture_v1";

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Probe(PhysicalReadAdmissionV1),
    Visit,
}

#[derive(Default)]
struct Trace {
    events: Vec<Event>,
    rows: Vec<(Vec<u8>, Vec<u8>)>,
}

fn namespace() -> StoreResult<JournalNamespace<'static>> {
    JournalNamespace::new(PHASE, "row/")
}

fn budget(rows: u64) -> ReadBudget {
    ReadBudget {
        rows,
        bytes: 4096,
        record_bytes: 1024,
    }
}

fn put(store: &Store, key: &str) -> StoreResult<()> {
    let mut batch = store.write_batch();
    batch.journal_done(
        PHASE,
        key,
        &json!({"synthetic": "namespace admission fixture"}),
    )?;
    batch.commit()?;
    Ok(())
}

#[test]
fn preadmitted_namespace_keeps_normal_and_empty_legacy_counts_bytes_and_fingerprint() -> TestResult
{
    use PhysicalReadAdmissionV1::{KeyProbe, Value, ValueSizeProbe};
    for count in [0, 2] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        for index in 0..count {
            put(&store, &format!("row/{index}"))?;
        }
        let snapshot = store.snapshot();
        let mut legacy = (0usize, Vec::new());
        let old = snapshot.scan_journal_namespace_contents(
            namespace()?,
            budget(count),
            &mut legacy,
            |state| {
                state.0 = state.0.checked_add(1).ok_or(StoreError::CounterOverflow)?;
                Ok(())
            },
            |state, key, raw| {
                state.1.push((key.to_vec(), raw.to_vec()));
                Ok(())
            },
        )?;
        check!(eq; legacy.0, usize::try_from(count)?);
        check!(eq; legacy.1.len(), usize::try_from(count)?);
        let mut trace = Trace::default();
        let new = snapshot.scan_journal_namespace_preadmitted(
            namespace()?,
            budget(count),
            &mut trace,
            |trace, event| {
                trace.events.push(Event::Probe(event));
                Ok(())
            },
            |trace, key, raw| {
                trace.events.push(Event::Visit);
                trace.rows.push((key.to_vec(), raw.to_vec()));
                Ok(())
            },
        )?;
        check!(eq; trace.rows, legacy.1);
        check!(eq; new.physical_rows(), old.physical_rows());
        check!(eq; new.encoded_bytes(), old.encoded_bytes());
        check!(eq; new.digest(), old.digest());
        let encoded = trace
            .rows
            .iter()
            .try_fold(0u64, |total, (key, raw)| -> TestResult<u64> {
                let bytes = key
                    .len()
                    .checked_add(raw.len())
                    .ok_or("fixture encoded bytes")?;
                Ok(total
                    .checked_add(u64::try_from(bytes)?)
                    .ok_or("fixture total bytes")?)
            })?;
        check!(eq; new.physical_rows(), count);
        check!(eq; new.encoded_bytes(), encoded);
        let expected = snapshot.journal_namespace_fingerprint(namespace()?, budget(count))?;
        let actual = JournalNamespaceFingerprint::from_contents(&snapshot, new);
        check!(expected.same_contents(&store, &actual)?);
        let mut events = vec![Event::Probe(KeyProbe)];
        for (key, raw) in &trace.rows {
            events.extend([
                Event::Probe(KeyProbe),
                Event::Probe(ValueSizeProbe),
                Event::Probe(Value {
                    key_bytes: key.len(),
                    value_bytes: raw.len(),
                }),
                Event::Visit,
            ]);
        }
        events.push(Event::Probe(KeyProbe));
        check!(eq; trace.events, events);
    }
    Ok(())
}

#[test]
fn namespace_probe_refusals_preserve_sentinel_before_visit_and_at_eof() -> TestResult {
    use PhysicalReadAdmissionV1::{KeyProbe, Value, ValueSizeProbe};
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    put(&store, "row/a")?;
    let snapshot = store.snapshot();
    let mut raw_rows = Vec::new();
    snapshot.scan_journal_namespace_contents(
        namespace()?,
        budget(1),
        &mut raw_rows,
        |_| Ok(()),
        |rows, key, raw| {
            rows.push((key.len(), raw.len()));
            Ok(())
        },
    )?;
    let [(key_bytes, value_bytes)] = raw_rows.as_slice() else {
        return Err("one actual retained row".into());
    };
    let expected = [
        KeyProbe,
        KeyProbe,
        ValueSizeProbe,
        Value {
            key_bytes: *key_bytes,
            value_bytes: *value_bytes,
        },
        KeyProbe,
    ];
    for stop in 1..=5 {
        let mut state = (Vec::new(), 0usize);
        let result = snapshot.scan_journal_namespace_preadmitted(
            namespace()?,
            budget(1),
            &mut state,
            |state, event| {
                state.0.push(event);
                if state.0.len() == stop {
                    return Err(StoreError::Refused {
                        detail: format!("namespace sentinel {stop}"),
                    });
                }
                Ok(())
            },
            |state, _, _| {
                state.1 = state.1.checked_add(1).ok_or(StoreError::CounterOverflow)?;
                Ok(())
            },
        );
        check!(
            matches!(result, Err(StoreError::Refused { detail }) if detail == format!("namespace sentinel {stop}"))
        );
        check!(eq; state.0.as_slice(), expected.get(..stop).ok_or("expected probe prefix")?);
        check!(eq; state.1, usize::from(stop == 5));
    }
    Ok(())
}

#[test]
fn namespace_preadmission_keeps_legacy_row_record_and_total_budget_error_order() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    put(&store, "row/a")?;
    let snapshot = store.snapshot();
    for limits in [
        ReadBudget {
            rows: 0,
            bytes: 0,
            record_bytes: 0,
        },
        ReadBudget {
            rows: 1,
            bytes: 0,
            record_bytes: 0,
        },
        ReadBudget {
            rows: 1,
            bytes: 0,
            record_bytes: 1024,
        },
    ] {
        let mut old_counts = (0usize, 0usize);
        let old = snapshot
            .scan_journal_namespace_contents(
                namespace()?,
                limits,
                &mut old_counts,
                |state| {
                    state.0 = state.0.checked_add(1).ok_or(StoreError::CounterOverflow)?;
                    Ok(())
                },
                |state, _, _| {
                    state.1 = state.1.checked_add(1).ok_or(StoreError::CounterOverflow)?;
                    Ok(())
                },
            )
            .err()
            .ok_or("legacy budget refusal")?;
        let mut trace = Trace::default();
        let new = snapshot
            .scan_journal_namespace_preadmitted(
                namespace()?,
                limits,
                &mut trace,
                |trace, event| {
                    trace.events.push(Event::Probe(event));
                    Ok(())
                },
                |trace, _, _| {
                    trace.events.push(Event::Visit);
                    Ok(())
                },
            )
            .err()
            .ok_or("preadmitted budget refusal")?;
        check!(eq; std::mem::discriminant(&new), std::mem::discriminant(&old));
        check!(eq; new.to_string(), old.to_string());
        check!(eq; old_counts, (1, 0));
        let expected = if limits.rows == 0 {
            vec![
                Event::Probe(PhysicalReadAdmissionV1::KeyProbe),
                Event::Probe(PhysicalReadAdmissionV1::KeyProbe),
            ]
        } else {
            vec![
                Event::Probe(PhysicalReadAdmissionV1::KeyProbe),
                Event::Probe(PhysicalReadAdmissionV1::KeyProbe),
                Event::Probe(PhysicalReadAdmissionV1::ValueSizeProbe),
            ]
        };
        check!(eq; trace.events, expected);
        match (limits.rows, limits.record_bytes, new) {
            (
                0,
                _,
                StoreError::BudgetExceeded {
                    what: "journal physical rows",
                    needed: 1,
                    available: 0,
                },
            ) => (),
            (1, 0, StoreError::RowTooLarge { bytes, max: 0 }) if bytes > 0 => (),
            (
                1,
                1024,
                StoreError::BudgetExceeded {
                    what: "journal encoded bytes",
                    needed,
                    available: 0,
                },
            ) if needed > 0 => (),
            _ => return Err("namespace budget order differs".into()),
        }
    }
    Ok(())
}
