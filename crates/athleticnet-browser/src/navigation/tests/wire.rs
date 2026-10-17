#[test]
fn wire_is_classified_before_decode_without_zero_default() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::transport::transfer::{classify_wire_total, refuse_excess_wire, WireTotal};
    use crate::BrowserError;
    check!(eq; classify_wire_total(0.0), WireTotal::WithinBudget);
    check!(eq; classify_wire_total(1_048_576.0), WireTotal::WithinBudget);
    check!(eq; classify_wire_total(40_000_000.0), WireTotal::Excess);
    check!(eq; classify_wire_total(f64::NAN), WireTotal::Malformed);
    check!(eq; classify_wire_total(f64::INFINITY), WireTotal::Malformed);
    check!(eq; classify_wire_total(-1.0), WireTotal::Malformed);
    check!(eq; refuse_excess_wire(Some(1_048_576.0)), Ok(()));
    check!(matches!(
        refuse_excess_wire(Some(40_000_000.0)),
        Err(BrowserError::PayloadLimit)
    ));
    check!(matches!(
        refuse_excess_wire(Some(f64::NAN)),
        Err(BrowserError::Protocol)
    ));
    check!(
        matches!(refuse_excess_wire(None), Err(BrowserError::Protocol)),
        "a missing total waits as protocol, never as zero"
    );
    Ok(())
}
