//! Published notation for a mark.
//!
//! Every formatting function works on integers only — no floating-point conversion — to agree with
//! the tree's independent renderer used in the property tests and to keep the crate's `as_cast`
//! count at zero. The construct is named rather than spelled because `xtask`'s per-line construct
//! counts read the raw line, so a comment that writes the cast out is scored as one.

use census_domain::model::CentiSeconds;
use census_domain::model::Mark;

/// Published notation for a mark: `10.94`, `4:41.23`, `5' 4"`, `42.10 m`, `3120 pts`.
pub fn mark_text(mark: &Mark) -> String {
    match mark {
        Mark::TimeSeconds(cs) => format_time(*cs),
        Mark::DistanceMetres(cm) => format_distance_metres(cm.0),
        Mark::FieldImperial { feet_mark, .. } => feet_mark.clone(),
        Mark::Points(cp) => format_points_scored(cp.0),
        Mark::Raw(text) => text.clone(),
    }
}

/// Centimetres as a published metre mark — `642` → `"6.42 m"`.
///
/// The unit is part of the notation a reader expects for a distance or field value, exactly as
/// `format_points_scored` carries `pts`; the comparable value is the integer, never this string.
pub fn format_distance_metres(cm: i32) -> String {
    let (whole, frac) = (cm / 100, (cm % 100).abs());
    format!("{whole}.{frac:02} m")
}

/// Centi-points as a scored string — `312000` → `"3120 pts"`.
pub fn format_points_scored(cp: i32) -> String {
    let (whole, _frac) = (cp / 100, (cp % 100).abs());
    format!("{whole} pts")
}

/// Centiseconds as a race time: `1094` stays `10.94`, `28123` becomes `4:41.23`,
/// `360000` becomes `1:00:00.00`.  Integer-only — no floating-point conversion — to agree
/// with the tree's independent renderer used in the property tests.
pub fn format_time(cs: CentiSeconds) -> String {
    let total_cs = cs.0.abs();
    let total_seconds = total_cs / 100;
    let sub_seconds = total_cs % 100;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}.{sub_seconds:02}")
    } else if minutes > 0 {
        format!("{minutes}:{seconds:02}.{sub_seconds:02}")
    } else {
        format!("{total_seconds}.{sub_seconds:02}")
    }
}
