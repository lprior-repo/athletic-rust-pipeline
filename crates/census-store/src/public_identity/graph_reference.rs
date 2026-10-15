use super::{
    paged_generation::{admission, CaptureBindingV2, ObjectRefV2},
    PublicIdentityError,
};
use crate::StoreResult;
use census_domain::model::serialized_digest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphAuthorityReferenceV1 {
    version: u32,
    #[serde(deserialize_with = "admission::decode_capture")]
    capture: CaptureBindingV2,
    #[serde(deserialize_with = "admission::decode_object")]
    seal: ObjectRefV2,
}

impl GraphAuthorityReferenceV1 {
    pub(in crate::public_identity) fn new(capture: CaptureBindingV2, seal: ObjectRefV2) -> Self {
        Self {
            version: 1,
            capture,
            seal,
        }
    }
    pub fn capture(&self) -> &CaptureBindingV2 {
        &self.capture
    }
    pub fn seal(&self) -> &ObjectRefV2 {
        &self.seal
    }
    pub fn domain_reference(
        &self,
    ) -> StoreResult<census_domain::public_identity_history::GraphAuthorityReferenceV1> {
        self.validate()?;
        census_domain::public_identity_history::GraphAuthorityReferenceV1::new(
            serialized_digest(&("public-identity-capture-binding/v1", &self.capture))?,
            self.seal.root().as_str().to_owned(),
        )
        .map_err(PublicIdentityError::from)
        .map_err(Into::into)
    }
    pub(in crate::public_identity) fn validate(&self) -> StoreResult<()> {
        if self.version != 1 {
            return Err(PublicIdentityError::Corrupt("graph reference version").into());
        }
        Ok(())
    }
    pub(in crate::public_identity) fn require_domain(
        &self,
        reference: &census_domain::public_identity_history::GraphAuthorityReferenceV1,
    ) -> StoreResult<()> {
        if &self.domain_reference()? != reference {
            return Err(PublicIdentityError::Corrupt("graph reference domain binding").into());
        }
        Ok(())
    }
}
