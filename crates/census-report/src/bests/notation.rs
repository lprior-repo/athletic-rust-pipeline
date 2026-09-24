//! Published notation for a mark.

use census_domain::model::Mark;
use census_domain::model::{CentiMetres, CentiPoints, CentiSeconds};

/// Published notation for a mark: `10.94`, `4:41.23`, `5' 4"`, `42.10 m`, `3120 pts`.
pub fn mark_text(mark: &Mark) -> String {
    match mark {
        Mark::TimeSeconds(cs) => format_time(*cs),
        Mark::DistanceMetres(cm) => format!("{:.2} m", cm.0 as f64 / 100.0),
        Mark::FieldImperial { feet_mark, .. } => feet_mark.clone(),
        Mark::Points(cp) => format!("{:.0} pts", cp.0 as f64 / 100.0),
        Mark::Raw(text) => text.clone(),
    }
}

/// Centiseconds as a race time: `1094` stays `10.94`, `28123` becomes `4:41.23`.
pub fn format_time(cs: CentiSeconds) -> String {
    let seconds = cs.0 as f64 / 100.0;
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
