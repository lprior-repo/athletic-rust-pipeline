use super::{defect, ReportResult};

const HEADERS: [&str; 36] = [
    "athlete_id",
    "name",
    "school",
    "state",
    "grad_year",
    "gender",
    "sport",
    "event",
    "surface",
    "wind_class",
    "timing_class",
    "best_mark",
    "original_mark",
    "best_value",
    "best_value_unit",
    "measure",
    "date",
    "meet",
    "place",
    "wind_mps",
    "timing",
    "marks_in_event",
    "profile_url",
    "performance_id",
    "source_athlete",
    "source_key",
    "comparison_policy",
    "comparison_specification",
    "source_specification",
    "adjustment_kind",
    "adjustment_basis",
    "adjustment_note",
    "adjustment_source",
    "eligible_candidate_count",
    "tied_performance_count",
    "tied_performance_ids",
];

#[derive(Clone, Copy)]
pub(super) enum Schema {
    Historical,
    CandidateInventory,
    ResultGroups,
}

impl Schema {
    pub(super) fn parse(artifact_revision: u32) -> ReportResult<Self> {
        match artifact_revision {
            1 | 2 => Ok(Self::Historical),
            3..=5 => Ok(Self::CandidateInventory),
            6..=8 => Ok(Self::ResultGroups),
            _ => Err(defect(format!(
                "unsupported publication artifact revision: {artifact_revision}"
            ))),
        }
    }

    pub(super) fn headers(self) -> ReportResult<Vec<&'static str>> {
        match self {
            Self::Historical => HEADERS
                .get(..33)
                .map(|headers| headers.to_vec())
                .ok_or_else(|| defect("historical best-results schema is incomplete".into())),
            Self::CandidateInventory => Ok(HEADERS.to_vec()),
            Self::ResultGroups => Ok(HEADERS
                .iter()
                .take(33)
                .copied()
                .chain([
                    "eligible_result_group_count",
                    "tied_result_group_count",
                    "tied_result_group_representative_ids",
                    "result_group_basis",
                    "physical_performance_count_status",
                    "selection_policy_version",
                ])
                .collect()),
        }
    }

    pub(super) fn project_json(
        self,
        mut expected: serde_json::Value,
    ) -> ReportResult<serde_json::Value> {
        if let Self::Historical = self {
            let fields = expected.as_object_mut().ok_or_else(|| {
                defect("shared best-results selection is not a JSON object".into())
            })?;
            for field in [
                "eligible_candidate_count",
                "tied_performance_count",
                "tied_performance_ids",
            ] {
                fields.remove(field);
            }
        }
        Ok(expected)
    }
}
