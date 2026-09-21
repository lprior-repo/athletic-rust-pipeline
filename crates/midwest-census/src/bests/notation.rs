//! Published notation for a mark.

use crate::model::Mark;

/// Published notation for a mark: `10.94`, `4:41.23`, `5' 4"`, `42.10 m`, `3120 pts`.
pub fn mark_text(mark: &Mark) -> String {
    match mark {
        Mark::TimeSeconds(seconds) => format_time(*seconds),
        Mark::DistanceMetres(metres) => format!("{metres:.2} m"),
        Mark::FieldImperial { feet_mark, .. } => feet_mark.clone(),
        Mark::Points(points) => format!("{points:.0} pts"),
        Mark::Raw(text) => text.clone(),
    }
}

/// Seconds as a race time: `10.94` stays `10.94`, `281.23` becomes `4:41.23`.
pub fn format_time(seconds: f64) -> String {
    if seconds < 60.0 {
        return format!("{seconds:.2}");
    }
    let minutes = (seconds / 60.0).floor();
    let remainder = seconds - minutes * 60.0;
    if remainder < 10.0 {
        format!("{minutes:.0}:0{remainder:.2}")
    } else {
        format!("{minutes:.0}:{remainder:.2}")
    }
}
