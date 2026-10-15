use super::{CatalogError, EventDisposition, EventExclusionReason, RecruitingCatalogV1};
use crate::model::{
    CanonicalEvent, EventResolutionReason, QueryProfileV1, SpecificationError, Sport,
};

impl RecruitingCatalogV1 {
    pub fn classify_for_profile(
        &self,
        event: &CanonicalEvent,
        sport: Sport,
        profile: &QueryProfileV1,
    ) -> Result<EventDisposition, CatalogError> {
        let disposition = self.classify(event, sport)?;
        if !matches!(disposition, EventDisposition::Included(_)) {
            return Ok(disposition);
        }
        let category = match self.selected_category(event) {
            Ok(category) => category,
            Err(reason) => return Ok(EventDisposition::Unresolved(reason)),
        };
        let input = profile.input();
        let excluded = match (
            input.sports.contains(&sport),
            input.competition_categories.contains(&category),
        ) {
            (true, true) => return Ok(disposition),
            (false, true) => EventExclusionReason::OutsideSelectedSport,
            (true, false) => EventExclusionReason::OutsideSelectedCompetitionCategory,
            (false, false) => EventExclusionReason::OutsideSelectedSportAndCompetitionCategory,
        };
        Ok(EventDisposition::Excluded(excluded))
    }

    fn selected_category(
        &self,
        event: &CanonicalEvent,
    ) -> Result<crate::model::Gender, EventResolutionReason> {
        let specification = event
            .resolved_specification_with_policy(self.semantics)
            .map_err(|error| match error {
                SpecificationError::ConflictingSpecification => {
                    EventResolutionReason::ContradictoryDeclarations
                }
                _ => EventResolutionReason::UnresolvedCategoryOrContext,
            })?;
        super::disposition::category(specification.category.as_ref(), event.gender)
    }
}

#[cfg(test)]
mod tests;
