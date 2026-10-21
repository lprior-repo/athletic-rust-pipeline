use crate::{StoreError, StoreResult};

#[cfg(test)]
mod tests;

pub(crate) mod child;
pub(crate) mod codec;
pub(crate) mod declaration;
mod directory;
pub(crate) mod intake;
pub use intake::{
    PreparedPublicSchoolIntakeV1, PublicSchoolIntakeAdmissionV1, PublicSchoolIntakeCaptureV1,
    PublicSchoolIntakeChildEvidenceV1, PublicSchoolIntakePacketV1, PublicSchoolIntakeRefV1,
};
pub(crate) mod format;
pub(crate) mod frontier;
pub(crate) mod read;
mod write;
pub(in crate::acquisition) use write::stage_entry;

pub use child::PreparedPublicRosterChildV1;
pub(crate) use declaration::PublicResourceDeclarationV1;
pub use declaration::{PublicResourceDeclarationBindingV1, PublicResourceOwnerBindingV1};
pub use directory::{
    PublicDirectoryEvidenceV1, PublicDirectoryRosterAssessmentV1, PublicDirectorySemanticContractV1,
};
pub(crate) use frontier::PublicResourceFrontierEntryV1;
pub use frontier::PublicResourceRootKindV1;

pub(crate) const DECLARATION: &str = "public_resource_capture/v1/declaration";
pub(crate) const ENTRY_PREFIX: &str = "public_resource_frontier/v1/entry/";
pub(crate) const OPERATION_PREFIX: &str = "public_resource_capture_v1:declare:";
pub(crate) const FRONTIER_OPERATION_PREFIX: &str = "public_resource_frontier_v1:";
pub(crate) const CHILD_PREFIX: &str = "public_resource_child_frontier/v1/entry/";
pub(crate) const CHILD_OPERATION_PREFIX: &str = "public_resource_child_frontier_v1:";

pub(super) fn refused(detail: &'static str) -> StoreError {
    StoreError::Refused {
        detail: detail.to_string(),
    }
}

pub(super) fn text(value: &str, limit: usize) -> StoreResult<()> {
    if value.is_empty()
        || value.len() > limit
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(refused("public resource bounded text"));
    }
    Ok(())
}
