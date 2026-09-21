//! How a mark is compared: one measure per mark scale, and the direction that wins in it.

use crate::model::Mark;

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

    /// The comparable number, in this measure's own unit.
    pub fn value(self, mark: &Mark) -> Option<f64> {
        match (self, mark) {
            (Measure::Time, Mark::TimeSeconds(seconds)) => Some(*seconds),
            (Measure::Distance, Mark::DistanceMetres(metres)) => Some(*metres),
            (Measure::Field, Mark::FieldImperial { metres, .. }) => Some(*metres),
            (Measure::Points, Mark::Points(points)) => Some(*points),
            _ => None,
        }
    }

    pub const fn better(self, candidate: f64, incumbent: f64) -> bool {
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
