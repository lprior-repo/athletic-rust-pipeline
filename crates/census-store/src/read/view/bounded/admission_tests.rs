use super::{PhysicalReadAdmissionV1, ReadBudget, ReadCounts};
use crate::{Entity, Store, StoreError};
use serde::{Deserialize, Serialize};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type Rows = Vec<(Row, Vec<u8>, Vec<u8>, u64)>;
const FIRST: &[u8] = b"schools\0alpha\0\x00\x00\x00\x00\x00\x00\x00\x00";
const SECOND: &[u8] = b"schools\0alpha\0\x00\x00\x00\x00\x00\x00\x00\x01";
const RAW: &[u8] = br#" { "value": 1, "id": "alpha", "unknown": true } "#;
const RAW_SECOND: &[u8] = br#"{"id":"alpha","value":2}"#;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Row {
    id: String,
    value: u8,
}

impl Entity for Row {
    fn entity_id(&self) -> &str { &self.id }
    fn merge(&mut self, other: Self) { self.value = other.value; }
}

#[derive(Debug, PartialEq, Eq)]
enum Event {
    Admission(PhysicalReadAdmissionV1),
    Visit(u8),
}

#[derive(Default)]
struct Trace {
    events: Vec<Event>,
    rows: Rows,
}

fn budget() -> ReadBudget {
    ReadBudget { rows: 2, bytes: 4096, record_bytes: 1024 }
}

fn value(key: &[u8], raw: &[u8]) -> PhysicalReadAdmissionV1 {
    PhysicalReadAdmissionV1::Value { key_bytes: key.len(), value_bytes: raw.len() }
}

#[test]
fn admitted_physical_read_orders_probes_values_and_visits_with_exact_bounded_parity() -> TestResult {
    use PhysicalReadAdmissionV1::{KeyProbe, ValueSizeProbe};
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.entities.insert(FIRST, RAW)?;
    store.entities.insert(SECOND, RAW_SECOND)?;
    let snapshot = store.snapshot();
    let mut trace = Trace::default();
    let counts = snapshot.for_each_physical_observation_admitted(
        crate::Table::Schools, budget(), &mut trace,
        |trace, event| { trace.events.push(Event::Admission(event)); Ok(()) },
        |trace, row: Row, key, raw, bytes| {
            trace.events.push(Event::Visit(row.value));
            trace.rows.push((row, key.to_vec(), raw.to_vec(), bytes));
            Ok(())
        },
    )?;
    check!(eq; trace.events, vec![
        Event::Admission(KeyProbe), Event::Admission(KeyProbe),
        Event::Admission(ValueSizeProbe), Event::Admission(value(FIRST, RAW)), Event::Visit(1),
        Event::Admission(KeyProbe), Event::Admission(ValueSizeProbe),
        Event::Admission(value(SECOND, RAW_SECOND)), Event::Visit(2), Event::Admission(KeyProbe),
    ]);
    let mut baseline = Rows::new();
    let baseline_counts = snapshot.for_each_physical_observation_bounded(
        crate::Table::Schools, budget(), |row: Row, key, raw, bytes| {
            baseline.push((row, key.to_vec(), raw.to_vec(), bytes)); Ok(())
        },
    )?;
    check!(eq; counts, baseline_counts);
    check!(eq; trace.rows, baseline);
    let first_bytes = u64::try_from(FIRST.len().checked_add(RAW.len()).ok_or("fixture bytes")?)?;
    let second_bytes = u64::try_from(SECOND.len().checked_add(RAW_SECOND.len()).ok_or("fixture bytes")?)?;
    let total = first_bytes.checked_add(second_bytes).ok_or("fixture total")?;
    check!(eq; trace.rows, vec![
        (Row { id: "alpha".into(), value: 1 }, FIRST.to_vec(), RAW.to_vec(), first_bytes),
        (Row { id: "alpha".into(), value: 2 }, SECOND.to_vec(), RAW_SECOND.to_vec(), total),
    ]);
    check!(eq; counts, ReadCounts { key_visits: 2, decoded_rows: 2, encoded_bytes: total, merged_rows: 0 });
    Ok(())
}

#[test]
fn admission_refusal_precedes_malformed_decode_and_visitor_at_each_physical_boundary() -> TestResult {
    use PhysicalReadAdmissionV1::{KeyProbe, ValueSizeProbe};
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let malformed = b"{!";
    store.entities.insert(FIRST, malformed.as_slice())?;
    let expected = [KeyProbe, KeyProbe, ValueSizeProbe, value(FIRST, malformed)];
    for stop in 1..=4 {
        let mut events = Vec::new();
        let result = store.snapshot().for_each_physical_observation_admitted::<Row, _>(
            crate::Table::Schools, budget(), &mut events,
            |events, event| {
                events.push(event);
                if events.len() == stop {
                    return Err(StoreError::Refused { detail: format!("admission sentinel {stop}") });
                }
                Ok(())
            },
            |_, _, _, _, _| Err(StoreError::Refused { detail: "visitor must not run".into() }),
        );
        check!(matches!(result, Err(StoreError::Refused { detail }) if detail == format!("admission sentinel {stop}")));
        check!(eq; events.as_slice(), expected.get(..stop).ok_or("expected probe prefix")?);
    }
    let mut events = Vec::new();
    let result = store.snapshot().for_each_physical_observation_admitted::<Row, _>(
        crate::Table::Schools, budget(), &mut events,
        |events, event| { events.push(event); Ok(()) },
        |_, _, _, _, _| Err(StoreError::Refused { detail: "visitor must not run".into() }),
    );
    check!(matches!(result, Err(StoreError::Decode { key, source })
        if key == "schools:alpha#0" && source.classify() == serde_json::error::Category::Syntax));
    check!(eq; events, expected);
    Ok(())
}

#[test]
fn empty_physical_scan_requires_initial_and_eof_admission_before_zero_counts() -> TestResult {
    use PhysicalReadAdmissionV1::KeyProbe;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    for stop in [Some(1), Some(2), None] {
        let mut events = Vec::new();
        let result = store.snapshot().for_each_physical_observation_admitted::<Row, _>(
            crate::Table::Schools, ReadBudget { rows: 0, bytes: 0, record_bytes: 0 }, &mut events,
            |events, event| {
                events.push(event);
                if Some(events.len()) == stop {
                    return Err(StoreError::Refused { detail: "empty scan admission sentinel".into() });
                }
                Ok(())
            },
            |_, _, _, _, _| Err(StoreError::Refused { detail: "empty visitor must not run".into() }),
        );
        match stop {
            Some(count) => {
                check!(matches!(result, Err(StoreError::Refused { detail }) if detail == "empty scan admission sentinel"));
                check!(eq; events, vec![KeyProbe; count]);
            }
            None => {
                check!(eq; result?, ReadCounts::default());
                check!(eq; events, vec![KeyProbe, KeyProbe]);
            }
        }
    }
    Ok(())
}
