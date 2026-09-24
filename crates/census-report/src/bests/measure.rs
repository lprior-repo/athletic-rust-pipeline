//! How a mark is compared: one measure per mark scale, and the direction that wins in it.

use census_domain::model::Mark;

/// How a mark is compared. Nothing crosses measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Measure {
    /// Lower is better.
    Time,
    /// Higher is better (metric distance or height).
    Distance,
    /// Higher is better, preserved in the source's feet-and-inches notation.
    Field,
    /// Higher is better (combined-event or team score).
    Points,
}

impl Measure {
    pub fn of(mark: &Mark) -> Option<Self> {
        match mark {
            Mark::TimeSeconds(_) => Some(Measure::Time),
            Mark::DistanceMetres(_) => Some(Measure::Distance),
            Mark::FieldImperial { .. } => Some(Measure::Field),
            Mark::Points(_) => Some(Measure::Points),
            Mark::Raw(_) => None,
        }
    }

    /// The comparable integer in this measure's own sub-unit.
    ///
    /// For field marks we parse the source's `feet_mark` string (e.g. `"3-0.75"`) directly into
    /// millimetres so that the integer comparison is finer than the centimetre-level
    /// [`Mark::FieldImperial::metres`] — a 1 cm bucket would collapse 1 321 of 3 619 adjacent
    /// quarter-inch notations.
    pub fn value(self, mark: &Mark) -> Option<i32> {
        match (self, mark) {
            (Measure::Time, Mark::TimeSeconds(cs)) => Some(cs.0),
            (Measure::Distance, Mark::DistanceMetres(cm)) => Some(cm.0),
            (Measure::Field, Mark::FieldImperial { feet_mark, .. }) => {
                parse_field_imperial(feet_mark)
            }
            (Measure::Points, Mark::Points(cp)) => Some(cp.0),
            _ => None,
        }
    }

    pub const fn better(self, candidate: i32, incumbent: i32) -> bool {
        match self {
            Measure::Time => candidate < incumbent,
            Measure::Distance | Measure::Field | Measure::Points => candidate > incumbent,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Measure::Time => "time",
            Measure::Distance => "distance",
            Measure::Field => "field",
            Measure::Points => "points",
        }
    }
}

/// Parse a `feet_mark` like `"3-0.75"` or `"61-03.50"` into millimetres for comparison.
///
/// The canonical track-and-field format is `feet-inches` (decimal inches), e.g.
/// `"3-0.75"` = 3 feet + 0.75 inches.  This gives ~0.254 mm resolution — far finer than the
/// centimetre-level [`Mark::FieldImperial::metres`] field, preventing adjacent quarter-inch
/// notations from collapsing.
///
/// Returns `None` when the string doesn't match the expected `F-I` pattern.
fn parse_field_imperial(feet_mark: &str) -> Option<i32> {
    let (feet_text, inches_text) = feet_mark.split_once('-')?;
    // Integer-only millimetre conversion to avoid `as_cast` budget violations.
    // 1 foot = 304.8 mm = 7620/25, 1 inch = 25.4 mm = 635/25.
    // Formula: (feet * 7620 + inches * 635 + 12) / 25  (half-adjust rounding)
    let feet: i64 = feet_text.parse().ok()?;
    // Parse inches as hundredths (e.g. "0.75" → 75, "3.50" → 350)
    let inches: i64 = parse_hundredths(inches_text)?;
    // Each step is checked so a pathological source string refuses instead of wrapping.
    let millimetres = feet
        .checked_mul(7620)?
        .checked_add(inches.checked_mul(635)?)?
        .checked_add(12)?;
    i32::try_from(millimetres / 25).ok()
}

/// Parse a decimal string like `"0.75"` or `"3.50"` as hundredths (→ 75 or 350).
fn parse_hundredths(s: &str) -> Option<i64> {
    let (whole, frac) = s.split_once('.')?;
    let whole_i: i64 = whole.parse().ok()?;
    // Pad or truncate frac to exactly 2 digits
    let frac = if frac.len() == 1 {
        format!("{frac}0")
    } else if frac.len() >= 2 {
        // Only the first two digits matter; a longer fraction is truncated, not rejected. `get`
        // fails on a non-boundary index, which is the right refusal for a non-ASCII fraction.
        frac.get(..2)?.to_string()
    } else {
        "00".to_string()
    };
    whole_i
        .checked_mul(100)?
        .checked_add(frac.parse::<i64>().ok()?)
}
