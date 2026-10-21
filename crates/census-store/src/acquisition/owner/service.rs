use super::PublicResourceCaptureOwnerV1;
use crate::acquisition::AcquisitionReadBudget;
use crate::{Store, StoreResult};
use census_domain::model::{AcquisitionDigest, ProfileSelectionV1};

impl PublicResourceCaptureOwnerV1 {
    pub fn require_service_binding(
        &self,
        store: &Store,
        selection: &ProfileSelectionV1,
        national_owner_key: &str,
        national_request_digest: &AcquisitionDigest,
        jurisdiction_request_digest: &AcquisitionDigest,
        source_plan_fingerprint: &AcquisitionDigest,
    ) -> StoreResult<()> {
        let _writes = store.lock_appends();
        let declaration = self.declaration(store, &mut AcquisitionReadBudget::for_work())?;
        let binding = &declaration.binding;
        let owner = binding.owner(self.scope.jurisdiction())?;
        if &binding.selection != selection
            || binding.national_owner_key != national_owner_key
            || &binding.national_request_digest != national_request_digest
            || &owner.jurisdiction_request_digest != jurisdiction_request_digest
            || &owner.source_plan_fingerprint != source_plan_fingerprint
        {
            return Err(super::super::public_resources::refused(
                "public resource service binding",
            ));
        }
        Ok(())
    }
}
