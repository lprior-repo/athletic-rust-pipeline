use super::super::SpecificationError;

mod decimal;

const MASS_UNITS: &[(&str, u64)] = &[
    ("kg", 1_000_000_000),
    ("pounds", 453_592_370),
    ("pound", 453_592_370),
    ("lbs", 453_592_370),
    ("lb", 453_592_370),
    ("g", 1_000_000),
];
const LENGTH_UNITS: &[(&str, u64)] = &[
    ("mm", 1_000),
    ("cm", 10_000),
    ("inches", 25_400),
    ("inch", 25_400),
    ("in", 25_400),
    ("\"", 25_400),
    ("m", 1_000_000),
];

pub(super) fn scaled(input: &str, multiplier: u64) -> Result<u64, SpecificationError> {
    let (coefficient, scale) = decimal::parts(input)?;
    let value = coefficient
        .checked_mul(multiplier)
        .ok_or(SpecificationError::InvalidLabel)?;
    if value
        .checked_rem(scale)
        .ok_or(SpecificationError::InvalidLabel)?
        != 0
    {
        return Err(SpecificationError::InvalidLabel);
    }
    value
        .checked_div(scale)
        .ok_or(SpecificationError::InvalidLabel)
}

pub(super) fn mass(input: &str, unit: Option<&str>) -> Result<u64, SpecificationError> {
    quantity(input, unit, MASS_UNITS)
}

pub(super) fn length(input: &str, unit: Option<&str>) -> Result<u64, SpecificationError> {
    quantity(input, unit, LENGTH_UNITS)
}

fn quantity(
    input: &str,
    unit: Option<&str>,
    units: &[(&str, u64)],
) -> Result<u64, SpecificationError> {
    let (number, multiplier) = match unit {
        Some(unit) => units
            .iter()
            .find(|(suffix, _)| unit.eq_ignore_ascii_case(suffix))
            .map(|(_, multiplier)| (input, *multiplier)),
        None => units.iter().find_map(|(suffix, multiplier)| {
            let split = input.len().checked_sub(suffix.len())?;
            let number = input.get(..split)?;
            input
                .get(split..)?
                .eq_ignore_ascii_case(suffix)
                .then_some((number, *multiplier))
        }),
    }
    .ok_or(SpecificationError::InvalidLabel)?;
    scaled(number.trim(), multiplier)
}
