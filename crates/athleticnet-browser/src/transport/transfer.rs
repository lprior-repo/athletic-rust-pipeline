use super::BrowserError;

pub(crate) const MAX_ENCODED_RESPONSE_BYTES_F64: f64 = 33_554_432.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WireTotal {
    WithinBudget,
    Excess,
    Malformed,
}

pub(crate) fn classify_wire_total(encoded_data_length: f64) -> WireTotal {
    if !encoded_data_length.is_finite() || encoded_data_length < 0.0 {
        return WireTotal::Malformed;
    }
    if encoded_data_length > MAX_ENCODED_RESPONSE_BYTES_F64 {
        return WireTotal::Excess;
    }
    WireTotal::WithinBudget
}

pub(crate) fn refuse_excess_wire(
    evaluation_ok: bool,
    encoded_total: Option<f64>,
) -> Result<(), BrowserError> {
    if !evaluation_ok {
        return Ok(());
    }
    let Some(total) = encoded_total else {
        return Err(BrowserError::Protocol);
    };
    match classify_wire_total(total) {
        WireTotal::WithinBudget => Ok(()),
        WireTotal::Excess => Err(BrowserError::PayloadLimit),
        WireTotal::Malformed => Err(BrowserError::Protocol),
    }
}

pub(crate) fn base64_len_for_decoded(decoded_len: u64) -> Option<u64> {
    decoded_len.checked_add(2)?.checked_div(3)?.checked_mul(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wire_total_below_the_encoded_cap_is_within_budget() {
        assert_eq!(classify_wire_total(0.0), WireTotal::WithinBudget);
        assert_eq!(classify_wire_total(1_048_576.0), WireTotal::WithinBudget);
        assert_eq!(
            classify_wire_total(MAX_ENCODED_RESPONSE_BYTES_F64),
            WireTotal::WithinBudget
        );
    }

    #[test]
    fn a_wire_total_above_the_encoded_cap_is_excess() {
        assert_eq!(
            classify_wire_total(MAX_ENCODED_RESPONSE_BYTES_F64 + 1.0),
            WireTotal::Excess
        );
        assert_eq!(classify_wire_total(1.0e12), WireTotal::Excess);
    }

    #[test]
    fn a_non_finite_or_negative_wire_total_is_malformed() {
        assert_eq!(classify_wire_total(f64::NAN), WireTotal::Malformed);
        assert_eq!(classify_wire_total(f64::INFINITY), WireTotal::Malformed);
        assert_eq!(classify_wire_total(f64::NEG_INFINITY), WireTotal::Malformed);
        assert_eq!(classify_wire_total(-1.0), WireTotal::Malformed);
    }

    #[test]
    fn a_successful_capture_with_a_measured_wire_total_is_accepted() {
        assert_eq!(refuse_excess_wire(true, Some(1_048_576.0)), Ok(()));
        assert_eq!(
            refuse_excess_wire(true, Some(MAX_ENCODED_RESPONSE_BYTES_F64)),
            Ok(())
        );
    }

    #[test]
    fn a_successful_capture_with_an_excess_wire_total_is_refused() {
        assert_eq!(
            refuse_excess_wire(true, Some(MAX_ENCODED_RESPONSE_BYTES_F64 + 1.0)),
            Err(BrowserError::PayloadLimit)
        );
    }

    #[test]
    fn a_successful_capture_without_a_wire_total_is_refused() {
        assert_eq!(refuse_excess_wire(true, None), Err(BrowserError::Protocol));
    }

    #[test]
    fn a_successful_capture_with_a_malformed_wire_total_is_refused() {
        assert_eq!(
            refuse_excess_wire(true, Some(f64::NAN)),
            Err(BrowserError::Protocol)
        );
        assert_eq!(
            refuse_excess_wire(true, Some(-1.0)),
            Err(BrowserError::Protocol)
        );
    }

    #[test]
    fn a_failed_evaluation_publishes_nothing_so_the_wire_gate_passes_through() {
        assert_eq!(refuse_excess_wire(false, None), Ok(()));
        assert_eq!(
            refuse_excess_wire(false, Some(MAX_ENCODED_RESPONSE_BYTES_F64 + 1.0)),
            Ok(())
        );
        assert_eq!(refuse_excess_wire(false, Some(f64::NAN)), Ok(()));
    }

    #[test]
    fn an_excess_wire_total_is_refused_even_when_the_decoded_body_is_small() {
        let small_decoded = 1024u64;
        let representation = base64_len_for_decoded(small_decoded);
        assert_eq!(representation, Some(1368));
        assert!(representation.is_some_and(|value| value < 33_554_432));
        assert_eq!(
            refuse_excess_wire(true, Some(40_000_000.0)),
            Err(BrowserError::PayloadLimit)
        );
    }

    #[test]
    fn the_base64_representation_is_larger_than_the_decoded_cap_it_carries() {
        let decoded_cap = u64::try_from(crate::protocol::MAX_SOURCE_RESPONSE_BYTES)
            .map_or(u64::MAX, |value| value);
        assert_eq!(decoded_cap, 33_554_432);
        let representation = base64_len_for_decoded(decoded_cap);
        assert_eq!(representation, Some(44_739_244));
        assert!(representation.is_some_and(|value| value > decoded_cap));
        assert!(representation.is_some_and(|value| value > 33_554_432));
    }

    #[test]
    fn base64_lengths_follow_the_quarter_block_expansion() {
        assert_eq!(base64_len_for_decoded(0), Some(0));
        assert_eq!(base64_len_for_decoded(1), Some(4));
        assert_eq!(base64_len_for_decoded(2), Some(4));
        assert_eq!(base64_len_for_decoded(3), Some(4));
        assert_eq!(base64_len_for_decoded(4), Some(8));
        assert_eq!(base64_len_for_decoded(u64::MAX), None);
    }
}
