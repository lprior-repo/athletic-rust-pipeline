use census_domain::model::{
    Grade, ObservedGrade, SourceAthleteObservation, SourceNamespace, SourceObservation, SourceRef,
};

use super::{AbsorbedMeet, RowWriter};
use crate::result_file::{ParsedEvent, ParsedRow};
use crate::{CrawlError, CrawlResult};

const MAX_UNDATED_PARTICIPANTS: usize = 250_000;

pub(super) fn retain(read: &AbsorbedMeet<'_>, writer: &mut RowWriter<'_>) -> CrawlResult<()> {
    for event in &read.parsed.events {
        for (ordinal, row) in event.rows.iter().enumerate() {
            let individual = row
                .legs
                .is_empty()
                .then_some((None, row.name.as_str(), row.grade));
            let members = individual.into_iter().chain(
                row.legs
                    .iter()
                    .map(|leg| (Some(leg.position), leg.name.as_str(), leg.grade)),
            );
            for member in members {
                retain_member(read, writer, event, row, ordinal, member)?;
            }
        }
    }
    Ok(())
}

fn retain_member(
    read: &AbsorbedMeet<'_>,
    writer: &mut RowWriter<'_>,
    event: &ParsedEvent,
    row: &ParsedRow,
    ordinal: usize,
    member: (Option<u8>, &str, Option<Grade>),
) -> CrawlResult<()> {
    let requested = writer
        .accumulator
        .undated
        .len()
        .checked_add(1)
        .ok_or_else(|| CrawlError::Arithmetic {
            detail: "WIAA undated participant count overflow".into(),
        })?;
    if requested > MAX_UNDATED_PARTICIPANTS {
        return Err(resource(requested));
    }
    writer
        .accumulator
        .undated
        .try_reserve(1)
        .map_err(|_| resource(requested))?;
    let (position, name, grade) = member;
    let key = super::map_rows::source_key(read.artifact, event, ordinal, position);
    let observed_grade = grade
        .zip(read.school_year)
        .map(|(grade, school_year)| ObservedGrade {
            grade,
            school_year,
            source: SourceRef::new("wiaa_results", Some(read.artifact.url.clone())),
        });
    let observation = SourceAthleteObservation::new(
        SourceNamespace::Other("wiaa_result_row".into()),
        key.clone(),
        key,
        name,
        read.observed_on,
    )
    .with_school((!row.school.trim().is_empty()).then(|| row.school.clone()))
    .with_grade(observed_grade)
    .with_gender(event.gender);
    writer
        .accumulator
        .undated
        .push(SourceObservation::Athlete(observation));
    Ok(())
}

fn resource(requested: usize) -> CrawlError {
    CrawlError::Resource {
        resource: "WIAA undated participants",
        requested,
        limit: MAX_UNDATED_PARTICIPANTS,
    }
}
