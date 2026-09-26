//! The three decisions one list row's own facts rest on: its grade, its mark and its key.

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

/// The mark a row publishes in the domain's own notation: a running mark as seconds, a field mark
/// in the host's own feet–inches notation beside its metric value.
///
/// The host's `Conv` column, when the row publishes one, is the host's own metric conversion and
/// wins over this reader's arithmetic; a field mark in no notation this reader can place stays
/// [`Mark::Raw`] rather than being guessed at.
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

/// The provider-local key of one listed mark.
///
/// View-independent on purpose: the same mark appears in the unfiltered list and in every grade view
/// of it, and both must upsert one performance. The columns a mark is identified by are its
/// date, the meet the row names (its numeric id when the row links one), the event (the host's own
/// standard-event handle when the section publishes one, its printed label otherwise), the
/// athlete's numeric id, the published mark and its place.
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
