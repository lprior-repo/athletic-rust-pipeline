use super::state::Stats;
use crate::tfrrs::parse::{
    clock_seconds, feet_inches_metres, ParsedMark, ParsedMeet, ParsedRow, ParsedSection,
    PublishedDate, YearToken,
};
use census_domain::model::{CentiMetres, Grade, Mark};

pub(super) fn grade_for(
    row: &ParsedRow,
    filter: Option<YearToken>,
    stats: &mut Stats,
) -> Option<Grade> {
    match row.year {
        Some(token) => {
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
        None => match filter.and_then(YearToken::grade) {
            Some(grade) => {
                stats.grades_from_filter = stats.grades_from_filter.saturating_add(1);
                Some(grade)
            }
            None => None,
        },
    }
}

pub(super) fn mark_of(mark: &ParsedMark, conv_metres: Option<f64>) -> Option<Mark> {
    match mark {
        ParsedMark::Time(token) => clock_seconds(token).map(Mark::TimeSeconds),
        ParsedMark::Field(token) => {
            if let Some(metres) = feet_inches_metres(token) {
                return Some(Mark::FieldImperial {
                    feet_mark: token.clone(),
                    metres: CentiMetres::try_from_metres_f64(metres)?,
                });
            }
            match conv_metres {
                Some(metres) => Some(Mark::DistanceMetres(CentiMetres::try_from_metres_f64(
                    metres,
                )?)),
                None => Some(Mark::Raw(token.clone())),
            }
        }
    }
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
