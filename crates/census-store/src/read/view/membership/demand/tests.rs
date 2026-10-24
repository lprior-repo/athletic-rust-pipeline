use crate::journal::{chunk_key, chunk_value, manifest_value};
use crate::read::{JournalNamespace, ReadBudget};
use crate::{Store, StoreError};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const PHASE: &str = "journal_demand_fixture_v1";

fn grant(rows: u64) -> ReadBudget {
    ReadBudget {
        rows,
        bytes: 67_108_864,
        record_bytes: 1_048_576,
    }
}

fn raw(store: &Store, key: &str, value: &[u8]) -> TestResult {
    store
        .journal
        .insert(Store::journal_key(PHASE, key), value)?;
    Ok(())
}

#[test]
fn diagnostic_visits_actual_25001_roots_while_normal_limits_still_refuse() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let mut batch = store.write_batch();
    for index in 0..25_001_u64 {
        batch.journal_done(PHASE, &format!("row/{index:05}"), &index)?;
    }
    batch.commit()?;
    let view = store.snapshot();
    let mut normal_visits = 0_u64;
    let normal = view.for_each_journal_root_bounded(
        JournalNamespace::new(PHASE, "row/")?,
        grant(25_000),
        |_| {
            normal_visits = normal_visits
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?;
            Ok(())
        },
    );
    check!(matches!(
        normal,
        Err(StoreError::BudgetExceeded {
            what: "journal physical rows",
            needed: 25_001,
            available: 25_000,
        })
    ));
    check!(eq; normal_visits, 25_000);
    check!(matches!(
        view.for_each_journal_root_bounded(
            JournalNamespace::new(PHASE, "row/")?,
            grant(25_001),
            |_| Ok(())
        ),
        Err(StoreError::BudgetExceeded {
            what: "journal admitted rows",
            needed: 25_001,
            available: 25_000,
        })
    ));
    let mut visits = 0_u64;
    let mut last = String::new();
    let complete = view.for_each_journal_root_demand_bounded(PHASE, grant(25_001), |key| {
        visits = visits.checked_add(1).ok_or(StoreError::CounterOverflow)?;
        key.clone_into(&mut last);
        Ok(())
    })?;
    check!(eq; visits, 25_001);
    check!(eq; last, "row/25000");
    check!(eq; complete.logical_roots(), 25_001);
    check!(eq; complete.fingerprint().physical_rows(), 25_001);
    check!(eq; complete.fingerprint().snapshot_sequence(), view.sequence());
    Ok(())
}

#[test]
fn diagnostic_admission_accepts_literal_ceilings_and_refuses_one_more() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let view = store.snapshot();
    let exact = ReadBudget {
        rows: 4_000_000,
        bytes: 17_179_869_184,
        record_bytes: 1_048_576,
    };
    let empty = view
        .for_each_journal_root_demand_bounded(PHASE, exact, |_| Err(StoreError::CounterOverflow))?;
    check!(eq; empty.logical_roots(), 0);
    check!(eq; empty.fingerprint().physical_rows(), 0);
    check!(eq; empty.fingerprint().encoded_bytes(), 0);
    for rows in [4_000_001, u64::MAX] {
        check!(matches!(view.for_each_journal_root_demand_bounded(
            PHASE, ReadBudget { rows, ..exact }, |_| Ok(())),
            Err(StoreError::BudgetExceeded { what: "journal demand admitted rows", needed, available: 4_000_000 })
                if needed == rows));
    }
    for bytes in [17_179_869_185, u64::MAX] {
        check!(matches!(view.for_each_journal_root_demand_bounded(
            PHASE, ReadBudget { bytes, ..exact }, |_| Ok(())),
            Err(StoreError::BudgetExceeded { what: "journal demand admitted bytes", needed, available: 17_179_869_184 })
                if needed == bytes));
    }
    for record_bytes in [1_048_577, usize::MAX] {
        check!(matches!(view.for_each_journal_root_demand_bounded(
            PHASE, ReadBudget { record_bytes, ..exact }, |_| Ok(())),
            Err(StoreError::RowTooLarge { bytes, max: 1_048_576 })
                if bytes == record_bytes));
    }
    Ok(())
}

#[test]
fn normal_byte_admission_remains_64_mib_after_diagnostic_admission() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let view = store.snapshot();
    let larger = ReadBudget {
        bytes: 67_108_865,
        ..grant(1)
    };
    check!(eq; view.for_each_journal_root_demand_bounded(PHASE, larger, |_| Ok(()))?.logical_roots(), 0);
    check!(matches!(
        view.for_each_journal_root_bounded(JournalNamespace::new(PHASE, "row/")?, larger, |_| Ok(
            ()
        )),
        Err(StoreError::BudgetExceeded {
            what: "journal admitted bytes",
            needed: 67_108_865,
            available: 67_108_864,
        })
    ));
    Ok(())
}

#[test]
fn diagnostic_counts_physical_chunks_once_per_logical_root_with_exact_bytes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let payload = br#"{"key":"row/a","payload":{"revision":1}}"#;
    let root_key = Store::journal_key(PHASE, "row/a");
    let root = manifest_value("row/a", 1, payload.len())?;
    let physical_chunk_key = chunk_key(&root_key, 0)?;
    let chunk = chunk_value("row/a", 0, 1, payload)?;
    raw(&store, "row/a", &root)?;
    store
        .journal
        .insert(physical_chunk_key.as_slice(), chunk.as_slice())?;
    let bytes = root_key
        .len()
        .checked_add(root.len())
        .and_then(|n| n.checked_add(physical_chunk_key.len()))
        .and_then(|n| n.checked_add(chunk.len()))
        .ok_or(StoreError::CounterOverflow)?;
    let exact = ReadBudget {
        rows: 2,
        bytes: u64::try_from(bytes)?,
        record_bytes: 1_048_576,
    };
    let view = store.snapshot();
    let mut seen = Vec::new();
    let complete = view.for_each_journal_root_demand_bounded(PHASE, exact, |key| {
        seen.push((key.to_string(), view.journal_payload(PHASE, key)?));
        Ok(())
    })?;
    check!(eq; seen, vec![("row/a".to_string(), Some(json!({"revision": 1})))]);
    check!(eq; complete.logical_roots(), 1);
    check!(eq; complete.fingerprint().physical_rows(), 2);
    check!(eq; complete.fingerprint().encoded_bytes(), exact.bytes);
    check!(matches!(
        view.for_each_journal_root_demand_bounded(PHASE, ReadBudget { rows: 1, ..exact }, |_| Ok(
            ()
        )),
        Err(StoreError::BudgetExceeded {
            what: "journal physical rows",
            needed: 2,
            available: 1
        })
    ));
    let short = exact
        .bytes
        .checked_sub(1)
        .ok_or(StoreError::CounterOverflow)?;
    check!(matches!(view.for_each_journal_root_demand_bounded(
        PHASE, ReadBudget { bytes: short, ..exact }, |_| Ok(())),
        Err(StoreError::BudgetExceeded { what: "journal encoded bytes", needed, available })
            if needed == exact.bytes && available == short));
    Ok(())
}

#[test]
fn incomplete_or_orphan_chunks_refuse_whole_phase_inventory() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    raw(&store, "row/a", &manifest_value("row/a", 1, 4)?)?;
    check!(
        matches!(store.snapshot().for_each_journal_root_demand_bounded(PHASE, grant(2), |_| Ok(())),
        Err(StoreError::JournalDescriptor { phase, key, detail })
            if phase == PHASE && key.is_empty() && detail == "logical root has missing physical chunks")
    );
    store.journal.remove(Store::journal_key(PHASE, "row/a"))?;
    store.journal.insert(
        chunk_key(&Store::journal_key(PHASE, "row/a"), 0)?,
        b"opaque".as_slice(),
    )?;
    check!(
        matches!(store.snapshot().for_each_journal_root_demand_bounded(PHASE, grant(2), |_| Ok(())),
        Err(StoreError::JournalDescriptor { phase, key, detail })
            if phase == PHASE && key.is_empty() && detail == "chunk has no preceding logical root")
    );
    Ok(())
}

#[test]
fn malformed_root_and_callback_errors_never_return_an_inventory() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    raw(&store, "row/a", b"{")?;
    check!(
        matches!(store.snapshot().for_each_journal_root_demand_bounded(PHASE, grant(1), |_| Ok(())),
        Err(StoreError::Decode { key, source })
            if key == format!("journal root type for {PHASE}/row/a") && source.is_eof())
    );
    raw(&store, "row/a", br#"{"key":"row/a","payload":null}"#)?;
    let mut visits = 0_u64;
    let failure = store
        .snapshot()
        .for_each_journal_root_demand_bounded(PHASE, grant(1), |_| {
            visits = visits.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            Err(StoreError::Refused {
                detail: "demand visitor stopped".into(),
            })
        });
    check!(
        matches!(failure, Err(StoreError::Refused { detail }) if detail == "demand visitor stopped")
    );
    check!(eq; visits, 1);
    Ok(())
}

#[test]
fn diagnostic_record_limit_is_checked_before_payload_decode_or_callback() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let value = br#"{"key":"row/a","payload":null}"#;
    raw(&store, "row/a", value)?;
    let exact = ReadBudget {
        record_bytes: value.len(),
        ..grant(1)
    };
    check!(eq; store.snapshot().for_each_journal_root_demand_bounded(PHASE, exact, |_| Ok(()))?.logical_roots(), 1);
    let short = value
        .len()
        .checked_sub(1)
        .ok_or(StoreError::CounterOverflow)?;
    check!(
        matches!(store.snapshot().for_each_journal_root_demand_bounded(
        PHASE, ReadBudget { record_bytes: short, ..exact }, |_| Err(StoreError::CounterOverflow)),
        Err(StoreError::RowTooLarge { bytes, max }) if bytes == value.len() && max == short)
    );
    raw(&store, "row/a", &vec![b'x'; 1_048_577])?;
    check!(matches!(
        store
            .snapshot()
            .for_each_journal_root_demand_bounded(PHASE, grant(1), |_| Err(
                StoreError::CounterOverflow
            )),
        Err(StoreError::RowTooLarge {
            bytes: 1_048_577,
            max: 1_048_576
        })
    ));
    Ok(())
}

#[test]
fn held_snapshot_excludes_later_roots_and_payload_changes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.journal_done(PHASE, "row/a", &1)?;
    let held = store.snapshot();
    let first = held.for_each_journal_root_demand_bounded(PHASE, grant(2), |_| Ok(()))?;
    store.journal_done(PHASE, "row/a", &2)?;
    store.journal_done(PHASE, "other/b", &3)?;
    store.journal_done("journal_demand_fixture_v10", "row/a", &4)?;
    let mut seen = Vec::new();
    let again = held.for_each_journal_root_demand_bounded(PHASE, grant(2), |key| {
        seen.push((key.to_string(), held.journal_payload(PHASE, key)?));
        Ok(())
    })?;
    check!(eq; seen, vec![("row/a".to_string(), Some(json!(1)))]);
    check!(first
        .fingerprint()
        .same_contents(&store, again.fingerprint())?);
    let current = store
        .snapshot()
        .for_each_journal_root_demand_bounded(PHASE, grant(2), |_| Ok(()))?;
    check!(eq; current.logical_roots(), 2);
    check!(!first
        .fingerprint()
        .same_contents(&store, current.fingerprint())?);
    Ok(())
}

#[test]
fn invalid_phase_and_zero_row_budget_refuse_before_root_decode() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    for phase in ["", "bad\nphase", "j2"] {
        check!(matches!(
            store
                .snapshot()
                .for_each_journal_root_demand_bounded(phase, grant(1), |_| Ok(())),
            Err(StoreError::Refused { .. })
        ));
    }
    raw(&store, "row/a", b"{")?;
    check!(matches!(
        store
            .snapshot()
            .for_each_journal_root_demand_bounded(PHASE, grant(0), |_| Ok(())),
        Err(StoreError::BudgetExceeded {
            what: "journal physical rows",
            needed: 1,
            available: 0
        })
    ));
    Ok(())
}
