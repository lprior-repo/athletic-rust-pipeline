//! Fixed-point wrappers for the numeric channels inside [`Mark`].
//!
//! All three store an exact integer in the source's native sub-unit, so equality and ordering are
//! integer comparisons — no floating-point rounding surprises.
//!
//! Wire form on the current writer is a raw integer in the stored sub-unit (e.g. `1094` for
//! 10.94 s). The reader also accepts the form the pre-migration writer emitted, a JSON *float* in
//! whole seconds (e.g. `10.94`), because rows persisted before the migration carry it and the
//! append-only store cannot be rewritten. The two forms are told apart by JSON number syntax and
//! each carries declared semantics:
//!
//! | Wire form    | Declared meaning                    | Example           |
//! |--------------|-------------------------------------|-------------------|
//! | JSON integer | centiseconds / centimetres / points | `1094` = 10.94 s  |
//! | JSON float   | whole units, scaled by `round(x*100)` | `10.94` = 10.94 s |
//!
//! Both forms fail closed. A value that does not fit in `i32` after scaling, and any non-finite
//! float, is a deserialization error — never a saturation and never a guessed unit. The float
//! constructors are fallible for the same reason, so no caller can turn NaN into a valid mark; the
//! integer constructor cannot fail, because every `i32` is already a valid stored sub-unit.

mod centi_distance;
mod centi_points;
mod centi_time;

pub use centi_distance::CentiMetres;
pub use centi_points::CentiPoints;
pub use centi_time::CentiSeconds;

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
