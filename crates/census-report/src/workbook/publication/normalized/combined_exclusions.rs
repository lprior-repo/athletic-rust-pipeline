use crate::bests::{CombinedExclusion, CombinedExclusionReason};
use crate::export::ExportDataset;
use crate::report::ReportResult;
use census_domain::model::{CombinedEligibility, SpecificationError};
use std::path::Path;

const HEADER: [&str; 7] = [
    "performance_id",
    "retained_athlete_id",
    "event_id",
    "reason_category",
    "reason_code",
    "selection_policy_version",
    "input_generation",
];

pub(super) fn write(
    directory: &Path,
    dataset: &ExportDataset,
    exclusions: &[CombinedExclusion],
    performances: &super::PerformanceIndex<'_>,
) -> ReportResult<u64> {
    super::write_csv(
        &directory.join("combined_exclusions.csv"),
        &HEADER,
        |writer| {
            exclusions.iter().try_for_each(|row| {
                let performance = performances.get(row.performance_id()).ok_or_else(|| {
                    super::defect("combined exclusion references an out-of-scope performance")
                })?;
                if &performance.athlete != row.athlete_id() || &performance.event != row.event_id()
                {
                    return Err(super::defect(
                        "combined exclusion references contradictory candidate facts",
                    ));
                }
                let (category, code) = reason(row.reason())?;
                writer.push([
                    row.performance_id().to_string(),
                    row.athlete_id().to_string(),
                    row.event_id().to_string(),
                    category.into(),
                    code.into(),
                    dataset.lineage.policy_revision.to_string(),
                    dataset.lineage.input_generation.clone(),
                ])
            })
        },
    )
}

fn reason(reason: CombinedExclusionReason) -> ReportResult<(&'static str, &'static str)> {
    match reason {
        CombinedExclusionReason::Eligibility(reason) => Ok((
            "eligibility",
            match reason {
                CombinedEligibility::Eligible => {
                    return Err(super::defect(
                        "eligible combined candidate cannot be excluded",
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
