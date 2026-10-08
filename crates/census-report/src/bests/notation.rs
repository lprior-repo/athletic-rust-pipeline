use census_domain::model::ExactSeconds;
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

pub fn format_time(time: ExactSeconds) -> String {
    let total_seconds = time.value() / 1_000_000_000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    if hours == 0 && minutes == 0 {
        return time.to_string();
    }
    let whole_seconds = total_seconds % 60;
    let exact = time.to_string();
    let fraction = exact.split_once('.').map(|(_, fraction)| fraction);
    let seconds = ClockSeconds {
        whole: whole_seconds,
        fraction,
    };
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds}")
    } else {
        format!("{minutes}:{seconds}")
    }
}

struct ClockSeconds<'a> {
    whole: i64,
    fraction: Option<&'a str>,
}

impl std::fmt::Display for ClockSeconds<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.fraction {
            Some(fraction) => write!(formatter, "{:02}.{fraction}", self.whole),
            None => write!(formatter, "{:02}", self.whole),
        }
    }
}

pub fn disagreement(meet: &str, marks: &[String]) -> String {
    format!("{meet}: {}", marks.join(" | "))
}
