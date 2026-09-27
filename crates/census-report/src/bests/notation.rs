
use census_domain::model::CentiSeconds;
use census_domain::model::Mark;

pub fn mark_text(mark: &Mark) -> String {
    match mark {
        Mark::TimeSeconds(cs) => format_time(*cs),
        Mark::DistanceMetres(cm) => format_distance_metres(cm.value()),
        Mark::FieldImperial { feet_mark, .. } => feet_mark.clone(),
        Mark::Points(cp) => format_points_scored(cp.value()),
        Mark::Raw(text) => text.clone(),
    }
}

pub fn format_distance_metres(cm: i32) -> String {
    let (whole, frac) = (cm / 100, (cm % 100).abs());
    format!("{whole}.{frac:02} m")
}

pub fn format_points_scored(cp: i32) -> String {
    let (whole, _frac) = (cp / 100, (cp % 100).abs());
    format!("{whole} pts")
}

pub fn format_time(cs: CentiSeconds) -> String {
    let total_cs = cs.value().abs();
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

pub fn disagreement(meet: &str, marks: &[String]) -> String {
    format!("{meet}: {}", marks.join(" | "))
}
