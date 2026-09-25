//! How a mark is compared: one measure per mark scale, and the direction that wins in it.

use census_domain::model::Mark;

/// Return a mark as the numeric value its workbook unit names.
///
/// The parsed wrappers retain hundredths, so the conversion returns seconds, metres, or points
/// with those hundredths represented after conversion. Raw notation has no comparable number.
pub fn mark_value(mark: &Mark) -> Option<f64> {
    match mark {
        Mark::TimeSeconds(value) => Some(value.as_seconds_f64()),
        Mark::DistanceMetres(value) => Some(value.as_metres_f64()),
        Mark::FieldImperial { metres, .. } => Some(metres.as_metres_f64()),
        Mark::Points(value) => Some(value.as_points_f64()),
        Mark::Raw(_) => None,
    }
}

/// Return the unit label paired with [`mark_value`], or no unit for raw notation.
pub fn mark_unit(mark: &Mark) -> Option<&'static str> {
    match mark {
        Mark::TimeSeconds(_) => Some("s"),
        Mark::DistanceMetres(_) | Mark::FieldImperial { .. } => Some("m"),
        Mark::Points(_) => Some("pts"),
        Mark::Raw(_) => None,
    }
}

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

/// Parse a `feet_mark` like `"3-0.75"`, `"61-03.50"`, `"145-09"` or `5' 4"` into millimetres for comparison.
///
/// The canonical track-and-field format is `feet-inches`, where the inches part is decimal
/// (`"3-0.75"` = 3 feet + 0.75 inches) or whole (`"145-09"` = 145 feet + 9 inches), and some hosts
/// publish `5' 4"` instead. This gives ~0.254 mm resolution — far finer than the centimetre-level
/// [`Mark::FieldImperial::metres`] field, preventing adjacent quarter-inch notations from collapsing.
///
/// Returns `None` when the string doesn't match a notation this reader can place.
fn parse_field_imperial(feet_mark: &str) -> Option<i32> {
    let (feet_text, inches_text) = split_feet_inches(feet_mark)?;
    let feet: i64 = feet_text.trim().parse().ok()?;
    let inches: i64 = parse_inches_hundredths(inches_text)?;
    // Work in hundredths of an inch: 1 foot = 12 inches = 1200, and 1 hundredth of an inch is
    // 0.254 mm, so millimetres = hundredths * 254 / 1000. Integer-only, like the rest of the
    // kernel, so no `as` cast is needed ([`crate::bests`] comparisons are exactly ordered).
    let hundredths = feet.checked_mul(1200)?.checked_add(inches)?;
    // Each step is checked so a pathological source string refuses instead of wrapping.
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
    // Pad or truncate frac to exactly 2 digits. A longer fraction is truncated, not rejected.
    // `get` fails on a non-boundary index, which is the right refusal for a non-ASCII fraction.
    let frac = match frac.len() {
        0 => 0,
        1 => frac.parse::<i64>().ok()?.checked_mul(10)?,
        _ => frac.get(..2)?.parse::<i64>().ok()?,
    };
    whole.checked_mul(100)?.checked_add(frac)
}
#[cfg(test)]
mod tests {
    use super::{mark_unit, mark_value, Measure};
    use census_domain::model::{CentiMetres, CentiPoints, CentiSeconds, Mark};

    #[test]
    fn mark_value_converts_each_published_scale() {
        assert_eq!(
            mark_value(&Mark::TimeSeconds(CentiSeconds(4855))),
            Some(48.55)
        );
        assert_eq!(
            mark_value(&Mark::DistanceMetres(CentiMetres(642))),
            Some(6.42)
        );
        assert_eq!(
            mark_value(&Mark::FieldImperial {
                feet_mark: "21-0.75".to_string(),
                metres: CentiMetres(642),
            }),
            Some(6.42)
        );
        assert_eq!(mark_value(&Mark::Points(CentiPoints(12345))), Some(123.45));
    }

    #[test]
    fn mark_value_and_unit_are_blank_for_raw_notation() {
        let mark = Mark::Raw("windy".to_string());
        assert_eq!(mark_value(&mark), None);
        assert_eq!(mark_unit(&mark), None);
    }

    #[test]
    fn mark_unit_matches_numeric_scale() {
        assert_eq!(mark_unit(&Mark::TimeSeconds(CentiSeconds(1))), Some("s"));
        assert_eq!(mark_unit(&Mark::DistanceMetres(CentiMetres(1))), Some("m"));
        assert_eq!(
            mark_unit(&Mark::FieldImperial {
                feet_mark: "1-0".to_string(),
                metres: CentiMetres(30),
            }),
            Some("m")
        );
        assert_eq!(mark_unit(&Mark::Points(CentiPoints(1))), Some("pts"));
    }

    #[test]
    fn field_marks_convert_to_exact_millimetres() {
        assert_eq!(millimetres("3-0.75"), 933, "3 ft 0.75 in");
        assert_eq!(millimetres("4-00"), 1_219, "4 ft exactly");
        assert_eq!(millimetres("145-09"), 44_425, "145 ft 9 in, whole inches");
        assert_eq!(millimetres("61-03.50"), 18_682, "61 ft 3.5 in");
        assert_eq!(millimetres("1-0"), 305, "1 ft exactly");
        assert_eq!(millimetres("5' 4\""), 1_626, "the apostrophe notation");
        assert_eq!(millimetres("6-06.25"), 1_988, "6 ft 6.25 in");
    }

    /// A mark with more feet outranks one with fewer, however many inches the shorter carries: the
    /// comparison follows the published notation itself, not a rescaled inch term.
    #[test]
    fn a_field_mark_with_more_feet_always_ranks_higher() {
        for (lower, higher) in [
            ("3-11.75", "4-00"),
            ("6-00", "6-00.25"),
            ("13-11.75", "14-00"),
            ("144-11.75", "145-00"),
        ] {
            assert!(
                millimetres(lower) < millimetres(higher),
                "{lower} ({}) must rank below {higher} ({})",
                millimetres(lower),
                millimetres(higher)
            );
        }
    }

    /// Cross-check the parse against the metric value every host publishes beside its feet-inches
    /// mark: agreement to the centimetre is what a 100x arithmetic slip cannot survive.
    #[test]
    fn parsed_feet_inches_agrees_with_the_published_metres() {
        for (feet_mark, centimetres) in [
            ("3-0.75", 93),
            ("5-04.25", 163),
            ("21-0.75", 642),
            ("61-03.50", 1_868),
            ("145-09", 4_442),
        ] {
            let parsed = millimetres(feet_mark);
            let published = centimetres * 10;
            assert!(
                (parsed - published).abs() <= 6,
                "{feet_mark} parses to {parsed} mm but the host publishes {published} mm"
            );
        }
    }

    /// A mark this reader can't place refuses instead of comparing on a wrong scale.
    #[test]
    fn an_unplaceable_field_mark_has_no_value() {
        assert_eq!(millimetres_opt("windy"), None);
        assert_eq!(millimetres_opt("-0.75"), None);
        assert_eq!(millimetres_opt("3-4-5"), None);
    }

    fn millimetres(feet_mark: &str) -> i32 {
        millimetres_opt(feet_mark).expect("a mark in the published notation parses")
    }

    fn millimetres_opt(feet_mark: &str) -> Option<i32> {
        Measure::value(
            Measure::Field,
            &Mark::FieldImperial {
                feet_mark: feet_mark.to_string(),
                metres: CentiMetres(0),
            },
        )
    }
}
