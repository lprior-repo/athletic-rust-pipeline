mod centi_distance;
mod centi_points;
mod exact_time;

pub use centi_distance::CentiMetres;
pub use centi_points::CentiPoints;
pub use exact_time::{ExactSeconds, TimeError};

fn checked_hundredths(value: f64) -> Option<i32> {
    rust_decimal::prelude::ToPrimitive::to_i32(&(value * 100.0).round())
}

fn display_hundredths(value: i32, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let decimal =
        rust_decimal::Decimal::try_new(i64::from(value), 2).map_err(|_| std::fmt::Error)?;
    write!(f, "{decimal}")
}

#[cfg(test)]
#[path = "fixed_mark_tests.rs"]
mod tests;
