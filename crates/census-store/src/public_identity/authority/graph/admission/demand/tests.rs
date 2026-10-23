use super::*;
use crate::public_identity_tests::support::TestResult;

#[test]
fn entry_total_is_inclusive_across_sections_and_overflow_preserves_prefix() -> TestResult {
    let literal = serde_json::json!({"key":"athlete:00000000000000000000","at":"",
        "payload":{"kind":"athlete","value":""}});
    let length = serde_json::to_vec(&literal)?
        .len()
        .checked_add(1)
        .ok_or("entry")?;
    let bytes = u64::try_from(length)?;
    let prior_sections = 34_359_738_368_u64.checked_sub(bytes).ok_or("entry total")?;
    let mut section = SectionDemand {
        table: AuthorityTableV2::Athletes,
        rows: 0,
        bytes: 0,
        max_entry_bytes: 0,
        scan: SectionScan::Provisional,
    };
    section.record("athlete", &"", prior_sections)?;
    check!(eq; section.rows, 1);
    check!(eq; section.bytes, bytes);
    let needed = 34_359_738_368_u64.checked_add(bytes).ok_or("overflow")?;
    match section.record("athlete", &"", prior_sections) {
        Err(StoreError::BudgetExceeded {
            what,
            needed: actual,
            available,
        }) => {
            check!(eq; what, "diagnostic authority entry bytes");
            check!(eq; actual, needed);
            check!(eq; available, 34_359_738_368);
        }
        _ => return Err("diagnostic total was not enforced".into()),
    }
    check!(eq; section.rows, 1);
    check!(eq; section.bytes, bytes);
    check!(eq; section.max_entry_bytes, length);
    Ok(())
}

#[test]
fn entry_total_overflow_is_refused_without_counting_record() -> TestResult {
    let mut section = SectionDemand {
        table: AuthorityTableV2::Athletes,
        rows: 0,
        bytes: 0,
        max_entry_bytes: 0,
        scan: SectionScan::Provisional,
    };
    check!(matches!(
        section.record("athlete", &"", u64::MAX),
        Err(StoreError::CounterOverflow)
    ));
    check!(eq; section.rows, 0);
    check!(eq; section.bytes, 0);
    Ok(())
}

#[test]
fn row_refusal_names_rows_and_preserves_seeded_prefix() -> TestResult {
    let mut section = SectionDemand {
        table: AuthorityTableV2::Athletes,
        rows: 2_097_152,
        bytes: 0,
        max_entry_bytes: 0,
        scan: SectionScan::Provisional,
    };
    match section.record("athlete", &"", 0) {
        Err(StoreError::BudgetExceeded {
            what,
            needed,
            available,
        }) => {
            check!(eq; what, "diagnostic authority section rows");
            check!(eq; needed, 2_097_153);
            check!(eq; available, 2_097_152);
        }
        _ => return Err("row refusal used the wrong unit".into()),
    }
    check!(eq; section.rows, 2_097_152);
    check!(eq; section.bytes, 0);
    Ok(())
}
