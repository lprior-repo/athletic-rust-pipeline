pub fn round_half_even(numerator: usize, denominator: usize, scale: usize) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let scaled = numerator.saturating_mul(scale);
    let quotient = scaled.checked_div(denominator).unwrap_or(0);
    let remainder = scaled.checked_rem(denominator).unwrap_or(0);
    let doubled = remainder.saturating_mul(2);
    let tie = doubled == denominator && quotient.checked_rem(2).unwrap_or(0) == 1;
    let adjusted = if doubled > denominator || tie {
        quotient.saturating_add(1)
    } else {
        quotient
    };
    let rounded = i32::try_from(adjusted).map_or(f64::from(i32::MAX), f64::from);
    let divisor = i32::try_from(scale).map_or(f64::from(i32::MAX), f64::from);
    rounded / divisor
}
