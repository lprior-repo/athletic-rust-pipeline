use super::super::{Discovery, MeetRoute, ResultsPlanError, ResultsPlanV2};
use super::text_limit;
use census_domain::model::{AcquisitionSlotV1, SourceMeetRef};

impl ResultsPlanV2 {
    pub(super) fn validate_meet(
        &self,
        meet: &SourceMeetRef,
        discovery: &Discovery,
        route: &MeetRoute,
    ) -> Result<(), ResultsPlanError> {
        super::super::wire::meets::strings(meet)
            .iter()
            .try_for_each(|text| text_limit(text))?;
        let scope = crate::restate_services::history_stage::HistoricalStageScope {
            jurisdiction: self.binding.scope().jurisdiction(),
            year: self.binding.scope().historical_year(),
            refresh: self.binding.scope().cache_intent()
                == census_domain::model::CaptureCacheIntent::AcquireNew,
            window: self.binding.scope().history(),
            observed_on: self.binding.scope().history().as_of(),
        };
        if meet.jurisdiction != scope.jurisdiction
            || super::super::super::selection::assess(meet, scope)
                != super::super::super::selection::Admission::Admitted
        {
            return Err(ResultsPlanError::Invalid("selected meet scope"));
        }
        let eligible = self.required_sources.iter().any(|slug| slug == "milesplit")
            && super::super::super::collection::eligible(
                super::super::super::ResultsArm::MilesplitResults,
                meet,
            );
        match route {
            MeetRoute::NotOwned if eligible => self.require_rejected_target(meet, discovery),
            MeetRoute::NotOwned => require_not_applicable(discovery),
            MeetRoute::Owned(_) if !eligible => {
                Err(ResultsPlanError::Invalid("foreign owned route"))
            }
            MeetRoute::Owned(index) => {
                let target = self
                    .targets
                    .get(index.position()?)
                    .ok_or(ResultsPlanError::Invalid("target index"))?;
                let slot = self.checked_slot(meet)?;
                if slot
                    .id()
                    .map_err(|_| ResultsPlanError::Invalid("slot digest"))?
                    != target.slot_id
                {
                    return Err(ResultsPlanError::Invalid("slot routing"));
                }
                validate_discovery(discovery, meet)
            }
        }
    }

    fn checked_slot(&self, meet: &SourceMeetRef) -> Result<AcquisitionSlotV1, ResultsPlanError> {
        let reference = super::super::super::capture_scope::checked_target(
            meet,
            &census_crawl::milesplit::ResultSetOptions::default(),
        )
        .map_err(|_| ResultsPlanError::Invalid("MileSplit reference"))?;
        let scope = self.binding.scope();
        AcquisitionSlotV1::new(
            self.binding.identity().clone(),
            scope.owner_key(),
            scope.historical_year(),
            reference.site.jurisdiction(),
            &reference.meet_id,
        )
        .map_err(|_| ResultsPlanError::Invalid("owned target"))
    }

    fn require_rejected_target(
        &self,
        meet: &SourceMeetRef,
        discovery: &Discovery,
    ) -> Result<(), ResultsPlanError> {
        let Discovery::Failed(failure) = discovery else {
            return Err(ResultsPlanError::Invalid("missing owned route"));
        };
        if failure.category != super::super::DiscoveryFailureCategory::TargetMismatch {
            return Err(ResultsPlanError::Invalid("unowned rejection category"));
        }
        if self.checked_slot(meet).is_ok() {
            return Err(ResultsPlanError::Invalid(
                "derivable target lacks owned route",
            ));
        }
        text_limit(&failure.detail)
    }
}

fn require_not_applicable(value: &Discovery) -> Result<(), ResultsPlanError> {
    match value {
        Discovery::NotApplicable => Ok(()),
        _ => Err(ResultsPlanError::Invalid("unowned discovery")),
    }
}

fn validate_discovery(value: &Discovery, meet: &SourceMeetRef) -> Result<(), ResultsPlanError> {
    match value {
        Discovery::NotApplicable => Err(ResultsPlanError::Invalid("missing owned discovery")),
        Discovery::Failed(failure) => text_limit(&failure.detail),
        Discovery::Ready(ready) => {
            if ready.pages_read > 1
                || ready.options.urls.len() > 4096
                || ready.quarantines.len() > 25
            {
                return Err(ResultsPlanError::Invalid("discovery cardinalities"));
            }
            super::super::super::capture_scope::checked_target(meet, &ready.options)
                .map_err(|_| ResultsPlanError::Invalid("discovery target mismatch"))?;
            for quarantine in &ready.quarantines {
                text_limit(&quarantine.url)?;
                text_limit(&quarantine.reason)?;
            }
            Ok(())
        }
    }
}
