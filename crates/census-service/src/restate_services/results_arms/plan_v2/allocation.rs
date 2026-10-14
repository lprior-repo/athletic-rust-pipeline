use super::{Discovery, MeetRoute, ResultsPlanError, ResultsPlanV2};

impl ResultsPlanV2 {
    pub(in crate::restate_services) fn serialization_allocation(
        &self,
    ) -> Result<usize, ResultsPlanError> {
        let targets = self
            .targets
            .len()
            .checked_mul(512)
            .ok_or(ResultsPlanError::Limit("allocation"))?;
        let pins = self
            .pins
            .len()
            .checked_mul(1024)
            .ok_or(ResultsPlanError::Limit("allocation"))?;
        targets
            .checked_add(pins)
            .and_then(|bytes| bytes.checked_add(4096))
            .ok_or(ResultsPlanError::Limit("allocation"))
    }

    pub(in crate::restate_services) fn validation_allocation(
        &self,
    ) -> Result<usize, ResultsPlanError> {
        let mut bytes = self.targets.len();
        for ((meet, route), discovery) in self.meets.iter().zip(&self.routes).zip(&self.discovery) {
            let count = super::codec::writer::count(meet)?;
            bytes = add(bytes, count)?;
            if matches!(route, MeetRoute::NotOwned)
                && matches!(discovery, Discovery::Failed(failure) if failure.category == super::DiscoveryFailureCategory::TargetMismatch)
            {
                bytes = add(bytes, 8192)?;
                bytes = add(bytes, reference_cost(&meet.results_url)?)?;
            }
            if matches!(route, MeetRoute::Owned(_)) {
                bytes = add(bytes, 8192)?;
                bytes = add(bytes, reference_cost(&meet.results_url)?)?;
                if let Discovery::Ready(ready) = discovery {
                    bytes = add(bytes, reference_cost(&meet.results_url)?)?;
                    for request in &ready.options.urls {
                        bytes = add(bytes, reference_cost(&request.url)?)?;
                    }
                }
            }
        }
        Ok(bytes)
    }
}

pub(super) fn reference_cost(url: &str) -> Result<usize, ResultsPlanError> {
    url.len()
        .checked_add(1)
        .and_then(|bytes| bytes.checked_mul(96))
        .ok_or(ResultsPlanError::Limit("reference allocation"))
}

fn add(left: usize, right: usize) -> Result<usize, ResultsPlanError> {
    left.checked_add(right)
        .ok_or(ResultsPlanError::Limit("allocation"))
}
