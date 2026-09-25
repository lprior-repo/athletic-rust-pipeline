//! How a mark is compared: one measure per mark scale, and the direction that wins in it.

mod notation;

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
            (Measure::Time, Mark::TimeSeconds(cs)) => Some(cs.value()),
            (Measure::Distance, Mark::DistanceMetres(cm)) => Some(cm.value()),
            (Measure::Field, Mark::FieldImperial { feet_mark, .. }) => {
                notation::parse_field_imperial(feet_mark)
            }
            (Measure::Points, Mark::Points(cp)) => Some(cp.value()),
            _ => None,
        }
    }

    pub const fn better(self, candidate: i32, incumbent: i32) -> bool {
        match self {
            Measure::Time => candidate < incumbent,
            Measure::Distance | Measure::Field | Measure::Points => candidate > incumbent,
        }
    }

    /// Convert the measure's integer value to its canonical f64 unit.
    pub fn normalized_mark(self, mark: &Mark) -> Option<f64> {
        self.value(mark).map(|v| match self {
            Measure::Time | Measure::Distance | Measure::Points => f64::from(v) / 100.0,
            Measure::Field => f64::from(v) / 1000.0,
        })
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

/// Parse a field mark string into millimetres; public for tests and callers that need
/// the fine-grained integer value without constructing a [`Mark`].
pub fn field_mm(feet_mark: &str) -> Option<i32> {
    notation::parse_field_imperial(feet_mark)
}
