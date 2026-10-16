use super::super::input::Inputs;
use super::rows;
use crate::bests::{CombinedExclusion, CombinedExclusionReason};
use crate::report::ReportResult;
use census_domain::model::{CombinedEligibility, SpecificationError};
use std::path::Path;

const HEADER: &[&str] = &[
    "performance_id",
    "retained_athlete_id",
    "event_id",
    "reason_category",
    "reason_code",
    "selection_policy_version",
    "input_generation",
];

pub(super) fn verify(
    directory: &Path,
    inputs: &Inputs<'_>,
    expected: &[CombinedExclusion],
) -> ReportResult<u64> {
    let candidate_column = if inputs.dataset.selected_profile_id().is_some() {
        "retained_candidate_id"
    } else {
        "retained_athlete_id"
    };
    let header: Vec<_> = HEADER
        .iter()
        .map(|name| {
            if *name == "retained_athlete_id" {
                candidate_column
            } else {
                *name
            }
        })
        .collect();
    rows::table(
        directory,
        ("combined_exclusions.csv", &header),
        expected.len(),
        &["performance_id"],
        |row| {
            let id = row.get("performance_id")?;
            let position = expected
                .binary_search_by(|value| value.performance_id().as_str().cmp(id))
                .map_err(|_| rows::missing("combined exclusion", id))?;
            let value = expected
                .get(position)
                .ok_or_else(|| rows::missing("combined exclusion", id))?;
            let (category, code) = reason(value.reason())?;
            row.text(candidate_column, value.athlete_id().as_str())?;
            row.text("event_id", value.event_id().as_str())?;
            row.text("reason_category", category)?;
            row.text("reason_code", code)?;
            row.number(
                "selection_policy_version",
                inputs.dataset.lineage.policy_revision,
            )?;
            row.text("input_generation", &inputs.dataset.lineage.input_generation)
        },
    )
}

fn reason(reason: CombinedExclusionReason) -> ReportResult<(&'static str, &'static str)> {
    match reason {
        CombinedExclusionReason::Eligibility(reason) => Ok((
            "eligibility",
            match reason {
                CombinedEligibility::Eligible => {
                    return Err(super::super::defect(
                        "eligible combined candidate cannot be excluded".into(),
                    ))
                }
                CombinedEligibility::MissingSpecification => "missing_specification",
                CombinedEligibility::MissingCompletion => "missing_completion",
                CombinedEligibility::SpecificationConflict => "specification_conflict",
                CombinedEligibility::Incomplete => "incomplete",
                CombinedEligibility::TotalMismatch => "total_mismatch",
                CombinedEligibility::PointsOverflow => "points_overflow",
            },
        )),
        CombinedExclusionReason::UnresolvedSpecification(reason) => Ok((
            "unresolved_specification",
            match reason {
                SpecificationError::InvalidDistance => "invalid_distance",
                SpecificationError::InvalidCourse => "invalid_course",
                SpecificationError::InvalidMass => "invalid_mass",
                SpecificationError::InvalidHurdles => "invalid_hurdles",
                SpecificationError::InvalidTrack => "invalid_track",
                SpecificationError::InvalidCategory => "invalid_category",
                SpecificationError::ConflictingSpecification => "conflicting_specification",
                SpecificationError::InvalidLabel => "invalid_label",
            },
        )),
    }
}
