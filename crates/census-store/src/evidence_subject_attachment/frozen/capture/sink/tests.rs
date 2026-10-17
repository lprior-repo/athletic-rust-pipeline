use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn count_accepts_exact_archive_bytes_without_large_allocation() -> TestResult {
    let mut count = Count::new();
    count.skip(
        MAX_ARCHIVE_BYTES
            .checked_sub(1)
            .ok_or(StoreError::CounterOverflow)?,
    )?;
    write(&mut count, b"]")?;
    check!(eq; count.finish()?, 536_870_912);
    Ok(())
}

#[test]
fn count_framing_overflow_preserves_primary_capacity_before_writer_wrapper() -> TestResult {
    let mut count = Count::new();
    count.skip(MAX_ARCHIVE_BYTES)?;
    check!(matches!(
        write(&mut count, b"]"),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "frozen archive writer refused"
        )))
    ));
    check!(count.write(b"secondary").is_err());
    check!(matches!(
        count.finish(),
        Err(StoreError::BudgetExceeded {
            what: "frozen attachment archive bytes",
            needed: 536_870_913,
            available: 536_870_912,
        })
    ));
    Ok(())
}

#[test]
fn count_serialization_overflow_preserves_capacity_before_json_wrapper() -> TestResult {
    let mut count = Count::new();
    count.skip(MAX_ARCHIVE_BYTES)?;
    check!(matches!(
        json(&mut count, &true),
        Err(StoreError::Json { .. })
    ));
    check!(count.write(b"secondary").is_err());
    check!(matches!(
        count.finish(),
        Err(StoreError::BudgetExceeded {
            what: "frozen attachment archive bytes",
            needed: 536_870_916,
            available: 536_870_912,
        })
    ));
    Ok(())
}

#[test]
fn small_output_preserves_exact_bytes_and_rejects_short_or_overrun_completion() -> TestResult {
    let mut exact = Output::new(3)?;
    write(&mut exact, b"a")?;
    write(&mut exact, b"bc")?;
    check!(eq; exact.finish()?, b"abc".to_vec());
    let mut short = Output::new(3)?;
    write(&mut short, b"ab")?;
    check!(matches!(
        short.finish(),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "frozen archive encoded length differs"
        )))
    ));
    let mut long = Output::new(3)?;
    write(&mut long, b"abc")?;
    check!(write(&mut long, b"d").is_err());
    check!(long.write(b"secondary").is_err());
    check!(matches!(
        long.finish(),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "frozen archive encoding overrun"
        )))
    ));
    Ok(())
}
