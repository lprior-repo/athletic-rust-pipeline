use super::*;
use crate::public_identity_tests::support::TestResult;

fn full_record_value() -> TestResult<String> {
    let entry = wire::Entry {
        key: "athlete:00000000000000000000",
        at: "",
        payload: wire::Payload {
            kind: "athlete",
            value: &"",
        },
    };
    let overhead = serde_json::to_vec(&entry)?
        .len()
        .checked_add(1)
        .ok_or("overflow")?;
    Ok("x".repeat(
        CONTEXT_RECORD_BYTES
            .checked_sub(overhead)
            .ok_or("overhead")?,
    ))
}

#[test]
fn retained_record_limit_includes_envelope_and_newline() -> TestResult {
    let value = full_record_value()?;
    measure(|sink| Section::new("athlete").record(sink, &value))?;
    let larger = format!("{value}x");
    check!(measure(|sink| Section::new("athlete").record(sink, &larger)).is_err());
    Ok(())
}

#[test]
fn retained_section_boundary_does_not_reset_per_row() -> TestResult {
    let value = full_record_value()?;
    let initial = BUDGET
        .bytes
        .checked_sub(u64::try_from(CONTEXT_RECORD_BYTES)?)
        .ok_or("section bound")?;
    measure(|sink| {
        let mut section = Section::new("athlete");
        section.bytes = initial;
        section.record(sink, &value)?;
        Ok(())
    })?;
    check!(measure(|sink| {
        let mut section = Section::new("athlete");
        section.bytes = initial;
        section.record(sink, &value)?;
        section.record(sink, &"")
    })
    .is_err());
    Ok(())
}

#[test]
fn aggregate_keeps_one_record_reserved_for_header() -> TestResult {
    check!(eq; CONTEXT_TOTAL_BYTES, 68_719_476_736);
    let exact = CONTEXT_TOTAL_BYTES
        .checked_sub(CONTEXT_RECORD_BYTES)
        .ok_or("reserve")?;
    aggregate(exact)?;
    check!(aggregate(exact.checked_add(1).ok_or("overflow")?).is_err());
    check!(aggregate(usize::MAX).is_err());
    Ok(())
}

#[test]
fn section_bytes_accept_exact_16_gib_and_refuse_one_more_byte() -> TestResult {
    let encoded = u64::try_from(entry_bytes("athlete", 0, &"")?)?;
    let initial = 17_179_869_184_u64
        .checked_sub(encoded)
        .ok_or("section entry")?;
    let mut sink = AuthorityBytes::default();
    let mut section = Section::new("athlete");
    section.bytes = initial;
    section.record(&mut sink, &"")?;
    check!(eq; section.bytes, 17_179_869_184);
    let mut excess = Section::new("athlete");
    excess.bytes = initial.checked_add(1).ok_or("section bytes")?;
    match excess.record(&mut AuthorityBytes::default(), &"") {
        Err(StoreError::BudgetExceeded {
            what,
            needed,
            available,
        }) => {
            check!(eq; what, "graph authority section bytes");
            check!(eq; needed, 17_179_869_185);
            check!(eq; available, 17_179_869_184);
        }
        _ => return Err("one-byte section excess was not refused".into()),
    }
    Ok(())
}

#[test]
fn four_full_sections_still_reserve_header_inside_64_gib_total() -> TestResult {
    let sections = usize::try_from(BUDGET.bytes)?
        .checked_mul(4)
        .ok_or("four sections")?;
    check!(eq; sections, 68_719_476_736);
    let needed = sections
        .checked_add(CONTEXT_RECORD_BYTES)
        .ok_or("header bytes")?;
    match aggregate(sections) {
        Err(StoreError::BudgetExceeded {
            what,
            needed: actual,
            available,
        }) => {
            check!(eq; what, "graph authority sections with header reserve");
            check!(eq; actual, u64::try_from(needed)?);
            check!(eq; available, 68_719_476_736);
        }
        _ => return Err("four full sections omitted the header reserve".into()),
    }
    aggregate(
        sections
            .checked_sub(CONTEXT_RECORD_BYTES)
            .ok_or("reserve")?,
    )?;
    Ok(())
}

#[test]
fn section_row_limit_is_cumulative() -> TestResult {
    let initial = BUDGET.rows.checked_sub(1).ok_or("row bound")?;
    measure(|sink| {
        let mut section = Section::new("athlete");
        section.rows = initial;
        section.record(sink, &"")?;
        Ok(())
    })?;
    check!(measure(|sink| {
        let mut section = Section::new("athlete");
        section.rows = initial;
        section.record(sink, &"")?;
        section.record(sink, &"")
    })
    .is_err());
    Ok(())
}

#[test]
fn aggregate_is_carried_across_sections_and_refusal_preserves_counters() -> TestResult {
    let mut probe = AuthorityBytes::default();
    Section::new("observation").record(&mut probe, &"")?;
    let encoded = probe.written_bytes();
    let exact = CONTEXT_TOTAL_BYTES
        .checked_sub(CONTEXT_RECORD_BYTES)
        .ok_or("reserve")?;
    let mut sink = AuthorityBytes {
        bytes: exact.checked_sub(encoded).ok_or("entry bytes")?,
    };
    let mut section = Section::new("observation");
    section.record(&mut sink, &"")?;
    check!(eq; sink.written_bytes(), exact);
    check!(eq; section.rows, 1);
    check!(eq; section.bytes, u64::try_from(encoded)?);
    let mut next = Section::new("observation");
    let needed = CONTEXT_TOTAL_BYTES
        .checked_add(encoded)
        .ok_or("aggregate bytes")?;
    match next.record(&mut sink, &"") {
        Err(StoreError::BudgetExceeded {
            what,
            needed: actual,
            available,
        }) => {
            check!(eq; what, "graph authority sections with header reserve");
            check!(eq; actual, u64::try_from(needed)?);
            check!(eq; available, u64::try_from(CONTEXT_TOTAL_BYTES)?);
        }
        _ => return Err("aggregate section overflow was not refused".into()),
    }
    check!(eq; sink.written_bytes(), exact);
    check!(eq; next.rows, 0);
    check!(eq; next.bytes, 0);
    Ok(())
}
