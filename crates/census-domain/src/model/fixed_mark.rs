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

/// Scale a decimal source value to integer hundredths, refusing anything that would not fit.
///
/// std has no checked conversion from a float to an integer, so the range test below is what lets
/// the one cast here be provably in range. It is the module's only cast, and it lives here so the
/// tree's `as_cast` budget has one documented site to read instead of fifteen scattered ones.
#[allow(clippy::as_conversions)]
fn checked_hundredths(value: f64) -> Option<i32> {
    let scaled = (value * 100.0).round();
    if !scaled.is_finite() || scaled < f64::from(i32::MIN) || scaled > f64::from(i32::MAX) {
        return None;
    }
    Some(scaled as i32)
}

#[cfg(test)]
#[path = "fixed_mark_tests.rs"]
mod tests;
