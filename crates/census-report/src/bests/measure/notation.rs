/// Parse a `feet_mark` like `"3-0.75"`, `"61-03.50"`, `"145-09"` or `5' 4"` into millimetres for comparison.
///
/// The canonical track-and-field format is `feet-inches`, where the inches part is decimal
/// (`"3-0.75"` = 3 feet + 0.75 inches) or whole (`"145-09"` = 145 feet + 9 inches), and some hosts
/// publish `5' 4"` instead. This gives ~0.254 mm resolution — far finer than the centimetre-level
/// [`census_domain::model::Mark::FieldImperial::metres`] field, preventing adjacent quarter-inch
/// notations from collapsing.
///
/// Returns `None` when the string doesn't match a notation this reader can place.
pub(crate) fn parse_field_imperial(feet_mark: &str) -> Option<i32> {
    let (feet_text, inches_text) = split_feet_inches(feet_mark)?;
    let feet: i64 = feet_text.trim().parse().ok()?;
    let inches: i64 = parse_inches_hundredths(inches_text)?;
    let hundredths = feet.checked_mul(1200)?.checked_add(inches)?;
    let millimetres = hundredths.checked_mul(254)?.checked_add(500)?;
    i32::try_from(millimetres / 1000).ok()
}

/// Split a published field mark into its feet and inches text: `3-0.75`, `145-09`, `5' 4"`.
fn split_feet_inches(feet_mark: &str) -> Option<(&str, &str)> {
    let trimmed = feet_mark.trim();
    let (feet, inches) = match trimmed.split_once('\'') {
        Some((feet, rest)) => (feet, rest),
        None => trimmed.split_once('-')?,
    };
    let inches = inches.trim().trim_end_matches('"').trim();
    Some((feet, inches))
}

/// Parse an inches field as hundredths of an inch: `"0.75"` → 75, `"3.50"` → 350, `"09"` → 900,
/// `""` → 0.
fn parse_inches_hundredths(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return Some(0);
    }
    let (whole, frac) = match s.split_once('.') {
        Some((whole, frac)) => (whole.trim(), frac),
        None => (s, ""),
    };
    let whole: i64 = whole.parse().ok()?;
    let frac = match frac.len() {
        0 => 0,
        1 => frac.parse::<i64>().ok()?.checked_mul(10)?,
        _ => frac.get(..2)?.parse::<i64>().ok()?,
    };
    whole.checked_mul(100)?.checked_add(frac)
}
