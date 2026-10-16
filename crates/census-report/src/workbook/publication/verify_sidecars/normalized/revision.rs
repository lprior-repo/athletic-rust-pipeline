use crate::bests::SharedSelection;
use crate::report::ReportResult;
use census_domain::model::PerformanceId;

#[derive(Clone, Copy)]
pub(super) enum Revision {
    RepresentativeOnly,
    SupportingReferences,
    CombinedExclusions,
    ResultGroups,
    ParticipationExclusions,
}

impl Revision {
    pub(super) fn parse(revision: u32) -> ReportResult<Self> {
        match revision {
            3 => Ok(Self::RepresentativeOnly),
            4 => Ok(Self::SupportingReferences),
            5 => Ok(Self::CombinedExclusions),
            6 => Ok(Self::ResultGroups),
            7 | 8 => Ok(Self::ParticipationExclusions),
            _ => Err(super::super::defect(format!(
                "unsupported normalized artifact revision {revision}"
            ))),
        }
    }

    pub(super) fn best_headers(self) -> Vec<&'static str> {
        let mut headers = super::schema::BESTS.to_vec();
        if self.has_supporting() {
            headers.push("supporting_tied_candidate_ids");
        }
        if self.has_result_groups() {
            headers.iter_mut().for_each(|name| {
                *name = match *name {
                    "eligible_candidate_count" => "eligible_result_group_count",
                    "tied_performance_count" => "tied_result_group_count",
                    "tied_performance_ids" => "tied_result_group_representative_ids",
                    other => other,
                }
            });
            headers.extend(["result_group_basis", "physical_performance_count_status"]);
        }
        headers
    }

    pub(super) fn has_supporting(self) -> bool {
        matches!(
            self,
            Self::SupportingReferences
                | Self::CombinedExclusions
                | Self::ResultGroups
                | Self::ParticipationExclusions
        )
    }

    pub(super) fn has_exclusions(self) -> bool {
        matches!(
            self,
            Self::CombinedExclusions | Self::ResultGroups | Self::ParticipationExclusions
        )
    }

    pub(super) fn has_participation_exclusions(self) -> bool {
        matches!(self, Self::ParticipationExclusions)
    }

    pub(super) fn has_result_groups(self) -> bool {
        matches!(self, Self::ResultGroups | Self::ParticipationExclusions)
    }

    pub(super) fn ids(self, best: &SharedSelection) -> &[PerformanceId] {
        match self {
            Self::RepresentativeOnly => best.inventory.tied_representative_ids(),
            Self::SupportingReferences
            | Self::CombinedExclusions
            | Self::ResultGroups
            | Self::ParticipationExclusions => best.supporting_tied_candidate_ids(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn participation_revision_preserves_the_result_group_projection(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let previous = Revision::parse(6)?;
        let selected = Revision::parse(7)?;
        check!(selected.has_result_groups());
        check!(eq; selected.best_headers(), previous.best_headers());
        check!(selected.has_supporting());
        check!(selected.has_exclusions());
        check!(selected.has_participation_exclusions());
        check!(!previous.has_participation_exclusions());
        Ok(())
    }
}
