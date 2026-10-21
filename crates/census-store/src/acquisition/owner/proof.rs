use super::{CaptureOwnerRef, PublicResourceCaptureScopeV1, ResultsCaptureScopeV1};
use census_domain::model::AcquisitionDigest;

pub(in crate::acquisition) enum OwnerScopeProof {
    Results(ResultsCaptureScopeV1),
    Public(PublicResourceCaptureScopeV1, AcquisitionDigest),
}

impl OwnerScopeProof {
    pub(in crate::acquisition) fn try_new(
        owner: CaptureOwnerRef<'_>,
    ) -> Result<Self, std::collections::TryReserveError> {
        match owner.0 {
            super::OwnerRef::Results(owner) => Ok(Self::Results(owner.scope.try_clone()?)),
            super::OwnerRef::Public(owner) => Ok(Self::Public(
                owner.scope.try_clone()?,
                owner.declaration_digest.try_clone()?,
            )),
        }
    }
    pub(in crate::acquisition) fn owned_bytes(owner: CaptureOwnerRef<'_>) -> usize {
        match owner.0 {
            super::OwnerRef::Results(owner) => owner.scope.owner_key().len() + 128,
            super::OwnerRef::Public(owner) => owner.scope.owner_key().len() + 320,
        }
    }
    pub(in crate::acquisition) fn matches(&self, owner: CaptureOwnerRef<'_>) -> bool {
        match (self, owner.0) {
            (Self::Results(scope), super::OwnerRef::Results(owner)) => scope == &owner.scope,
            (Self::Public(scope, digest), super::OwnerRef::Public(owner)) => {
                scope == &owner.scope && digest == &owner.declaration_digest
            }
            _ => false,
        }
    }
}
