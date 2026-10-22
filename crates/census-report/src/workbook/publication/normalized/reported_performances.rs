use crate::report::{ReportError, ReportResult};
use crate::workbook::reported::Projection;
use census_domain::model::CanonicalPerformance;
use std::path::Path;

pub(in crate::workbook::publication) const HEADER: &[&str] = &[
    "Record Basis",
    "Verification Status",
    "Canonical Athlete ID",
    "Source Person IDs",
    "Name (canonical record)",
    "Graduation Year (canonical record)",
    "Source Team IDs",
    "School at performance",
    "State at performance",
    "Performance ID",
    "Event ID",
    "Event",
    "Meet ID",
    "Meet",
    "Date",
    "Mark",
    "Timing",
    "Wind",
    "Place",
    "Source Key",
    "Performance Evidence",
    "Retained Conflicts",
    "Reported Event Interpretation",
];

pub(super) fn write(directory: &Path, projection: &Projection<'_>) -> ReportResult<u64> {
    super::write_csv(
        &directory.join("reported_performances.csv"),
        HEADER,
        |rows| {
            for performance in &projection.performances {
                rows.push(record(projection, performance)?)?;
            }
            Ok(())
        },
    )
}

fn record(
    projection: &Projection<'_>,
    performance: &CanonicalPerformance,
) -> ReportResult<[String; 23]> {
    let athlete =
        projection
            .athlete(&performance.athlete)
            .ok_or_else(|| ReportError::Invariant {
                detail: "reported performance has no exact canonical athlete parent".into(),
            })?;
    let team = projection.source.teams.get(&performance.team);
    let school = team.and_then(|team| projection.source.schools.get(&team.school));
    let event = projection.event(&performance.event);
    let meet = projection.meet(&performance.meet);
    Ok([
        "source_reported".into(),
        "reported_unverified_needs_review".into(),
        performance.athlete.as_str().into(),
        json(
            &athlete
                .identities()
                .chain(performance.source_athlete.iter())
                .collect::<Vec<_>>(),
        )?,
        athlete.canonical_name.clone(),
        athlete.grad_year.get().to_string(),
        json(&team.map(|team| &team.source_identities))?,
        school.map_or_else(String::new, |school| school.name.clone()),
        school
            .and_then(|school| school.state)
            .map_or_else(String::new, |state| state.code().into()),
        performance.id.as_str().into(),
        performance.event.as_str().into(),
        event.map_or_else(String::new, |event| event.kind.stable_key().into_owned()),
        performance.meet.as_str().into(),
        meet.map_or_else(String::new, |meet| meet.name.clone()),
        performance.date.clone(),
        crate::bests::mark_text(&performance.mark),
        performance
            .timing
            .map_or_else(String::new, |timing| timing.stable_key().into()),
        performance
            .wind_mps
            .map_or_else(String::new, |wind| wind.to_string()),
        performance
            .place
            .map_or_else(String::new, |place| place.to_string()),
        performance.source_key.clone(),
        json(&performance.evidence)?,
        json(&performance.retained_conflicts)?,
        match projection.interpretation(performance)? {
            Some(interpretation) => json(interpretation)?,
            None => String::new(),
        },
    ])
}

fn json(value: &impl serde::Serialize) -> ReportResult<String> {
    serde_json::to_string(value).map_err(|error| ReportError::Invariant {
        detail: format!("encoding reported performance provenance: {error}"),
    })
}
