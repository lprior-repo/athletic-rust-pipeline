mod notation;

use census_domain::model::Mark;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Measure {
    Time,
    Distance,
    Points,
}

impl Measure {
    pub fn of(mark: &Mark) -> Option<Self> {
        match mark {
            Mark::TimeSeconds(_) => Some(Measure::Time),
            Mark::DistanceMetres(_) => Some(Measure::Distance),
            Mark::FieldImperial { .. } => Some(Measure::Distance),
            Mark::Points(_) => Some(Measure::Points),
            Mark::Raw(_) => None,
        }
    }

    pub fn value(self, mark: &Mark) -> Option<i64> {
        match (self, mark) {
            (Measure::Time, Mark::TimeSeconds(cs)) => Some(i64::from(cs.value())),
            (Measure::Distance, Mark::DistanceMetres(cm)) => Some(i64::from(cm.value()) * 10_000),
            (Measure::Distance, Mark::FieldImperial { feet_mark, .. }) => {
                notation::parse_field_imperial(feet_mark)
            }
            (Measure::Points, Mark::Points(cp)) => Some(i64::from(cp.value())),
            _ => None,
        }
    }

    pub const fn better(self, candidate: i64, incumbent: i64) -> bool {
        match self {
            Measure::Time => candidate < incumbent,
            Measure::Distance | Measure::Points => candidate > incumbent,
        }
    }

    pub fn normalized_mark(self, mark: &Mark) -> Option<f64> {
        let value = self.value(mark)?;
        let (scale, floating_scale) = match self {
            Measure::Time | Measure::Points => (100, 100.0),
            Measure::Distance => (1_000_000, 1_000_000.0),
        };
        let units = i32::try_from(value / scale).ok()?;
        let fraction = i32::try_from(value % scale).ok()?;
        Some((f64::from(units) * floating_scale + f64::from(fraction)) / floating_scale)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Measure::Time => "time",
            Measure::Distance => "distance",
            Measure::Points => "points",
        }
    }
}

pub fn mark_unit(mark: &Mark) -> Option<&'static str> {
    match mark {
        Mark::TimeSeconds(_) => Some("s"),
        Mark::DistanceMetres(_) | Mark::FieldImperial { .. } => Some("m"),
        Mark::Points(_) => Some("pts"),
        Mark::Raw(_) => None,
    }
}

pub fn field_micrometres(feet_mark: &str) -> Option<i64> {
    notation::parse_field_imperial(feet_mark)
}
