use super::*;
use crate::{ReadBudget, Store};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn whole_phase_preserves_phase_validation_and_exact_key_boundary() -> TestResult {
    check!(JournalNamespace::new("phase", "").is_err());
    for phase in ["", "phase\0suffix", "phase\n"] {
        check!(JournalNamespace::whole_phase(phase).is_err());
    }
    let object = std::str::from_utf8(crate::journal::OBJECT_NAMESPACE)?;
    check!(JournalNamespace::whole_phase(object).is_err());
    let phase = "p".repeat(
        MAX_JOURNAL_KEY_BYTES
            .checked_sub(1)
            .ok_or(StoreError::CounterOverflow)?,
    );
    let namespace = JournalNamespace::whole_phase(&phase)?;
    check!(eq; namespace.phase(), phase);
    check!(eq; namespace.logical_prefix(), "");
    check!(eq; namespace.physical_prefix()?.len(), MAX_JOURNAL_KEY_BYTES);
    check!(JournalNamespace::whole_phase(&format!("{phase}p")).is_err());
    Ok(())
}

#[test]
fn complete_phase_inventory_includes_roots_outside_known_record_prefix() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.journal_done("whole_phase_test", "attachment/v1/record/1", &true)?;
    store.journal_done("whole_phase_test", "unknown-owned-root", &false)?;
    store.journal_done("whole_phase_test_other", "foreign", &true)?;
    let snapshot = store.snapshot();
    let mut keys = Vec::new();
    snapshot.for_each_journal_root_bounded(
        JournalNamespace::whole_phase("whole_phase_test")?,
        ReadBudget {
            rows: 4,
            bytes: 16 * 1024,
            record_bytes: 4096,
        },
        |key| {
            keys.push(key.to_owned());
            Ok(())
        },
    )?;
    check!(eq; keys, ["attachment/v1/record/1", "unknown-owned-root"]);
    Ok(())
}
