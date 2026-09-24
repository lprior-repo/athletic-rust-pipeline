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
    pub fn value(self, mark: &Mark) -> Option<i32> {
        match (self, mark) {
            (Measure::Time, Mark::TimeSeconds(cs)) => Some(cs.0),
            (Measure::Distance, Mark::DistanceMetres(cm)) => Some(cm.0),
            (Measure::Field, Mark::FieldImperial { metres, .. }) => Some(metres.0),
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
