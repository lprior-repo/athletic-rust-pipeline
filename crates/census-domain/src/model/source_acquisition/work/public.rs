use super::*;
use crate::model::PublicResourceObligationV1;

impl AcquisitionWorkV1 {
    pub fn for_public_resource(
        identity: StoreIdentity,
        scope: &PublicResourceCaptureScopeV1,
        site: UsJurisdiction,
        obligation: PublicResourceObligationV1,
        request: RequestRepresentationV1,
    ) -> Result<Self, CaptureDomainError> {
        let slot = AcquisitionSlotV1::for_public_resource(
            identity,
            scope.owner_key(),
            site,
            obligation.resource().clone(),
        )?;
        let result = Self {
            version: VersionOne,
            slot,
            run: scope.run(),
            run_manifest_digest: scope.run_manifest_digest().clone(),
            history: scope.history(),
            source_admission: scope.source_admission().clone(),
            request,
            cache_intent: scope.cache_intent(),
            protocol: CaptureProtocol::SourceAcquisitionAttemptV1,
            public_resource_obligation: Some(Box::new(obligation)),
        };
        result.validate()?;
        Ok(result)
    }
    pub fn public_resource_obligation(&self) -> Option<&PublicResourceObligationV1> {
        self.public_resource_obligation.as_deref()
    }
    pub(super) fn validate_public_resource(&self) -> Result<(), CaptureDomainError> {
        let obligation =
            self.public_resource_obligation
                .as_ref()
                .ok_or(CaptureDomainError::Invalid {
                    field: "work resource obligation",
                })?;
        let as_of = chrono::NaiveDate::parse_from_str(obligation.as_of().as_str(), "%Y-%m-%d")
            .map_err(|_| CaptureDomainError::HistoryConflict)?;
        if self.slot.public_resource() != Some(obligation.resource())
            || !self.source_admission.is_public_resources()
            || as_of != self.history.as_of()
            || !self.request.observed_http_required()
        {
            return Err(CaptureDomainError::Invalid {
                field: "work resource obligation",
            });
        }
        obligation
            .resource()
            .validate_target(self.slot.source_site(), self.request.original_url())
    }
}

#[cfg(test)]
mod tests;
