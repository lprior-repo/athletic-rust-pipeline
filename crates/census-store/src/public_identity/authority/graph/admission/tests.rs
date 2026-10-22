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
    let exact = CONTEXT_TOTAL_BYTES
        .checked_sub(CONTEXT_RECORD_BYTES)
        .ok_or("reserve")?;
    aggregate(exact)?;
    check!(aggregate(exact.checked_add(1).ok_or("overflow")?).is_err());
    check!(aggregate(usize::MAX).is_err());
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
