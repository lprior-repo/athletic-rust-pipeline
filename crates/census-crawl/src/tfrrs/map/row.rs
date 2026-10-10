use super::state::Stats;
use crate::tfrrs::parse::{
    clock_seconds, feet_inches_metres, metric_metres, ParsedMark, ParsedMeet, ParsedRow,
    ParsedSection, PublishedDate, YearToken,
};
use census_domain::model::{CentiMetres, Grade, Mark};

pub(super) fn grade_for(
    row: &ParsedRow,
    filter: Option<YearToken>,
    stats: &mut Stats,
) -> Option<Grade> {
    match row.year {
        Some(token) => published_grade(token, filter, stats),
        None => {
            if filter.and_then(YearToken::grade).is_some() {
                stats.grade_less_with_filter = stats.grade_less_with_filter.saturating_add(1);
            }
            None
        }
    }
}

fn published_grade(
    token: YearToken,
    filter: Option<YearToken>,
    stats: &mut Stats,
) -> Option<Grade> {
    if filter.is_some_and(|filter| filter != token) {
        stats.grades_conflicting_filter = stats.grades_conflicting_filter.saturating_add(1);
    }
    match token.grade() {
        Some(grade) => Some(grade),
        None => {
            stats.rows_below_high_school = stats.rows_below_high_school.saturating_add(1);
            None
        }
    }
}

pub fn mark_of(mark: &ParsedMark, conv_metres: Option<f64>) -> Mark {
    match mark {
        ParsedMark::Time(token) => match clock_seconds(token) {
            Some(seconds) => Mark::TimeSeconds(seconds),
            None => Mark::Raw(token.clone()),
        },
        ParsedMark::Field(token) => field_mark_of(token, conv_metres),
    }
}

fn field_mark_of(token: &str, conv_metres: Option<f64>) -> Mark {
    if let Some(metres) = feet_inches_metres(token) {
        return field_mark(
            token,
            CentiMetres::try_from_metres_f64(metres),
            conv_metres,
            FieldSystem::Imperial,
        );
    }
    if let Some(metres) = metric_metres(token) {
        return field_mark(
            token,
            CentiMetres::try_from_metres_f64(metres),
            conv_metres,
            FieldSystem::Metric,
        );
    }
    conv_mark(token, conv_metres)
}

fn conv_mark(token: &str, conv_metres: Option<f64>) -> Mark {
    match conv_metres {
        Some(metres) => match CentiMetres::try_from_metres_f64(metres) {
            Some(converted) => Mark::DistanceMetres(converted),
            None => Mark::Raw(token.to_string()),
        },
        None => Mark::Raw(token.to_string()),
    }
}

enum FieldSystem {
    Imperial,
    Metric,
}

fn field_mark(
    token: &str,
    primary: Option<CentiMetres>,
    conv_metres: Option<f64>,
    system: FieldSystem,
) -> Mark {
    let Some(primary) = primary else {
        return Mark::Raw(token.to_string());
    };
    if conv_contradicts(primary, conv_metres) {
        return Mark::Raw(token.to_string());
    }
    match system {
        FieldSystem::Imperial => Mark::FieldImperial {
            feet_mark: token.to_string(),
            metres: primary,
        },
        FieldSystem::Metric => Mark::DistanceMetres(primary),
    }
}

fn conv_contradicts(primary: CentiMetres, conv_metres: Option<f64>) -> bool {
    conv_metres
        .and_then(CentiMetres::try_from_metres_f64)
        .is_some_and(|conv| conv.value().abs_diff(primary.value()) > 2)
}

pub(super) fn source_key(
    date: &PublishedDate,
    meet: &ParsedMeet,
    athlete_id: Option<u64>,
    section: &ParsedSection,
    row: &ParsedRow,
) -> String {
    let meet_ref = meet.id.map_or_else(
        || format!("n{}", meet.name.to_lowercase()),
        |id| format!("m{id}"),
    );
    let athlete_ref = athlete_id.map_or_else(|| "-".to_string(), |id| id.to_string());
    let event_ref = section
        .event_hnd
        .map_or_else(|| section.label.clone(), |handle| format!("e{handle}"));
    let mark = match row.mark.as_ref() {
        Some(ParsedMark::Time(token) | ParsedMark::Field(token)) => token.as_str(),
        None => "-",
    };
    let place = row
        .place
        .map_or_else(|| "-".to_string(), |place| place.to_string());
    format!(
        "{}:{}:{}:{}:{}:{}",
        date.iso, meet_ref, event_ref, athlete_ref, mark, place
    )
}
pub(super) fn admit_date(
    page: super::state::Page<'_>,
    row: &ParsedRow,
) -> crate::CrawlResult<bool> {
    let Some(date) = row.date.as_ref() else {
        return Err(crate::CrawlError::PerformanceDateUnknown {
            published: String::new(),
            as_of: page.performance_as_of,
        });
    };
    match crate::context::assess_performance_date(page.performance_as_of, &date.iso) {
        crate::context::PerformanceDateAssessment::Admitted => Ok(true),
        crate::context::PerformanceDateAssessment::Future => Ok(false),
        crate::context::PerformanceDateAssessment::Unknown => {
            Err(crate::CrawlError::PerformanceDateUnknown {
                published: date.raw.chars().take(64).collect(),
                as_of: page.performance_as_of,
            })
        }
    }
}
