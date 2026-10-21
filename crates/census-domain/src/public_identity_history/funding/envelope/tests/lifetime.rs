use super::*;

#[test]
fn parent_is_held_across_grant_bytes_and_text_then_dropped_once() -> TestResult {
    let (budget, observer) = bounded_budget(16)?;
    check!(eq; observer.get(), 0);
    let (bytes, grant) = {
        let mut funding = budget.funding();
        let mut bytes = OwnedHistoryBytes::with_capacity(8, &mut funding)?;
        let grant = funding.reserve(storage(4)?)?;
        bytes.extend_from_slice(b"history")?;
        check!(eq; observer.get(), 0);
        (bytes, grant)
    };
    check!(eq; observer.get(), 0);
    let text = bytes.into_text()?;
    check!(eq; text.as_text(), "history");
    check!(eq; observer.get(), 0);
    drop(grant);
    check!(eq; observer.get(), 0);
    drop(text);
    check!(eq; observer.get(), 0);
    drop(budget);
    check!(eq; observer.get(), 1);
    Ok(())
}
