use super::*;

#[test]
fn confidence_new_admits_exactly_zero_to_one_hundred() -> Result<(), Box<dyn std::error::Error>> {
    for value in 0..=100u8 {
        check!(eq; Confidence::new(value).map(Confidence::get), Some(value));
    }
    for value in 101..=u8::MAX {
        check!(
            Confidence::new(value).is_none(),
            "Confidence::new({value}) must refuse a value above 100"
        );
    }
    Ok(())
}

#[test]
fn confidence_try_from_refuses_the_same_values_as_new() -> Result<(), Box<dyn std::error::Error>> {
    for value in 0..=100u8 {
        check!(eq; Confidence::try_from(value)?.get(), value);
    }
    for value in 101..=u8::MAX {
        let refused = matches!(
            Confidence::try_from(value),
            Err(ConfidenceError::OutOfRange { value: refused }) if refused == value
        );
        check!(
            refused,
            "Confidence::try_from({value}) must report the refused value"
        );
    }
    Ok(())
}

#[test]
fn confidence_deserialization_admits_the_range_and_keeps_accepted_bytes(
) -> Result<(), Box<dyn std::error::Error>> {
    for value in 0..=100u8 {
        let text = value.to_string();
        let parsed: Confidence = serde_json::from_str(&text)?;
        check!(eq; parsed.get(), value);
        check!(
            eq;
            serde_json::to_string(&parsed)?,
            text,
            "an accepted confidence must serialize back to the bytes it was read from"
        );
    }
    for value in 101..=u8::MAX {
        let refused = serde_json::from_str::<Confidence>(&value.to_string()).is_err();
        check!(refused, "deserialization must refuse confidence {value}");
    }
    for text in ["-1", "256", "65535", "\"50\"", "null", "1.0"] {
        let refused = serde_json::from_str::<Confidence>(text).is_err();
        check!(refused, "deserialization must refuse {text}");
    }
    Ok(())
}
