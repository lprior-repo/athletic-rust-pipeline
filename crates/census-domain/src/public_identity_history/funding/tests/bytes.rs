use super::*;

#[test]
fn text_conversion_retains_the_funded_buffer_until_text_drop() -> TestResult {
    let mut funding = TestFunding::new(16);
    let pool = Rc::clone(funding.pool());
    let mut bytes = OwnedHistoryBytes::with_capacity(8, &mut funding)?;
    bytes.extend_from_slice("écho".as_bytes())?;
    let original = bytes.as_bytes().as_ptr();
    check!(eq; pool.live(), 8);
    drop(funding);
    let text = bytes.into_text()?;
    check!(eq; text.as_text(), "écho");
    check!(eq; text.as_bytes().as_ptr(), original);
    check!(eq; text.capacity(), 8);
    check!(eq; pool.available(), 8);
    check!(eq; pool.live(), 8);
    drop(text);
    check!(eq; pool.available(), 16);
    check!(eq; pool.live(), 0);
    check!(pool.valid());
    Ok(())
}

#[test]
fn failed_extension_and_one_byte_short_reservation_preserve_prior_value() -> TestResult {
    let mut funding = TestFunding::new(7);
    let mut first = OwnedHistoryBytes::with_capacity(4, &mut funding)?;
    first.extend_from_slice(b"keep")?;
    check!(matches!(
        first.extend_from_slice(b"!"),
        Err(HistoryStorageError::Capacity {
            needed: 5,
            capacity: 4
        })
    ));
    check!(matches!(
        OwnedHistoryBytes::with_capacity(4, &mut funding),
        Err(HistoryStorageError::Funding(Fault::Limit))
    ));
    check!(eq; first.as_bytes(), b"keep");
    check!(eq; funding.pool().available(), 3);
    check!(eq; funding.pool().live(), 4);
    check!(eq; funding.calls(), 2);
    drop(first);
    check!(eq; funding.pool().available(), 7);
    check!(funding.pool().valid());
    Ok(())
}

#[test]
fn malformed_utf8_releases_its_owned_charge() -> TestResult {
    let mut funding = TestFunding::new(8);
    let mut bytes = OwnedHistoryBytes::with_capacity(4, &mut funding)?;
    bytes.extend_from_slice(&[0xf0, 0x28, 0x8c, 0x28])?;
    check!(eq; funding.pool().live(), 4);
    check!(matches!(
        bytes.into_text(),
        Err(HistoryStorageError::InvalidUtf8)
    ));
    check!(eq; funding.pool().live(), 0);
    check!(eq; funding.pool().available(), 8);
    check!(funding.pool().valid());
    Ok(())
}

#[test]
fn empty_bytes_and_text_need_no_grant() -> TestResult {
    let mut funding = TestFunding::new(0);
    let mut bytes = OwnedHistoryBytes::with_capacity(0, &mut funding)?;
    bytes.extend_from_slice(b"")?;
    check!(matches!(
        bytes.extend_from_slice(b"x"),
        Err(HistoryStorageError::Capacity {
            needed: 1,
            capacity: 0
        })
    ));
    let text = bytes.into_text()?;
    check!(text.is_empty());
    check!(eq; text.capacity(), 0);
    check!(eq; funding.calls(), 0);
    check!(eq; funding.pool().live(), 0);
    Ok(())
}

#[test]
fn impossible_byte_layout_refuses_without_requesting_a_grant() -> TestResult {
    let mut funding = TestFunding::new(16);
    check!(matches!(
        OwnedHistoryBytes::with_capacity(usize::MAX, &mut funding),
        Err(HistoryStorageError::Overflow)
    ));
    check!(eq; funding.calls(), 0);
    check!(eq; funding.pool().available(), 16);
    Ok(())
}
