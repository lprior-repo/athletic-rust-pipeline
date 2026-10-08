use super::super::super::SpecificationError;

pub(super) fn parts(input: &str) -> Result<(u64, u64), SpecificationError> {
    let (whole, fraction) = match input.split_once('.') {
        Some(parts) => parts,
        None => (input, ""),
    };
    validate(whole, fraction, input.len())?;
    let exponent = u32::try_from(fraction.len()).map_err(|_| SpecificationError::InvalidLabel)?;
    let scale = 10_u64
        .checked_pow(exponent)
        .ok_or(SpecificationError::InvalidLabel)?;
    Ok((coefficient(whole, fraction, scale)?, scale))
}

fn coefficient(whole: &str, fraction: &str, scale: u64) -> Result<u64, SpecificationError> {
    let whole = whole
        .parse::<u64>()
        .map_err(|_| SpecificationError::InvalidLabel)?;
    let fractional = match fraction {
        "" => 0,
        value => value
            .parse::<u64>()
            .map_err(|_| SpecificationError::InvalidLabel)?,
    };
    whole
        .checked_mul(scale)
        .and_then(|value| value.checked_add(fractional))
        .ok_or(SpecificationError::InvalidLabel)
}

fn validate(whole: &str, fraction: &str, length: usize) -> Result<(), SpecificationError> {
    if whole.is_empty()
        || fraction.len() > 9
        || length > 24
        || !whole.bytes().all(|ch| ch.is_ascii_digit())
        || !fraction.bytes().all(|ch| ch.is_ascii_digit())
    {
        return Err(SpecificationError::InvalidLabel);
    }
    Ok(())
}
