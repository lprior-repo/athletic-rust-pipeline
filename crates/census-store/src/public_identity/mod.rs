pub(crate) mod graph_operation;
pub(crate) mod graph_reference;
pub(crate) mod graph_witness;
use census_domain::public_identity_history::{PublicAthleteId, RegistryHead};
pub use graph_reference::GraphAuthorityReferenceV1;

mod attachment_fence;
pub(crate) use attachment_fence::read_head as attachment_public_head;
mod authority;
mod codec;
mod commit;
mod context;
mod correction;
mod local_adjudication;
pub(crate) use local_adjudication::admission::{
    audit_integrity as audit_manual_admission, owns_integrity_phase as is_manual_admission_phase,
};
pub use local_adjudication::{
    LocalAdjudicationCommitOutcomeV1, LocalAdjudicationInventoryV1, PreparedLocalAdjudicationV1,
};
pub(crate) mod frozen;
mod init;
#[cfg(test)]
pub(crate) use init::create_historical_test_fixture;
mod input_revisions;
mod inventory;
mod limits;
pub(crate) use limits::REPLAY_LIMITS;
mod membership;
mod membership_withholding;
pub use membership::{SourceCandidateMembershipOutcomeV1, SourceCandidateMembershipRequestV1};
pub use membership_withholding::{
    MembershipWithholdingInventoryV1, MembershipWithholdingOutcomeV1,
    PreparedMembershipWithholdingV1,
};
mod operation;
pub mod paged_generation;
pub(crate) mod qualification;
mod retained;
mod witness;

pub(crate) const LIFECYCLE: u32 =
    census_domain::public_identity_qualification::PUBLIC_IDENTITY_QUALIFICATION_LIFECYCLE;
pub(crate) const RETAINED_CONTEXT_LIFECYCLE: u32 =
    census_domain::public_identity_qualification::PUBLIC_IDENTITY_QUALIFICATION_V2_LIFECYCLE;
pub(crate) const GRAPH_AUTHORITY_LIFECYCLE: u32 =
    census_domain::public_identity_qualification::PUBLIC_IDENTITY_QUALIFICATION_V3_LIFECYCLE;
pub(crate) const OPERATION_PREFIX: &str = "public_identity_v1:";
pub(crate) const DECLARATION: &str = "public_identity:v1:qualification";
const HEAD: &str = "public_identity:v1:head";
const HISTORY: &str = "public_identity:v1:history";
pub const MAX_PUBLIC_IDENTITY_HISTORY_BYTES: usize = 256 * 1024 * 1024;

pub use correction::{ReviewedIdentityCommitOutcomeV1, ReviewedIdentityRequestV1};
pub use frozen::{
    replay_frozen_public_identity_v1, replay_frozen_public_identity_v2,
    FrozenPublicIdentityBundleV2, MAX_FROZEN_PUBLIC_IDENTITY_BYTES,
};
pub(crate) use qualification::{preflight_keyspaces, require_legacy, validate};
pub use qualification::{PublicIdentityQualificationStore, PublicIdentityWriterV1};

pub(crate) fn is_qualification_lifecycle(revision: Option<u32>) -> bool {
    matches!(
        revision,
        Some(
            LIFECYCLE
                | RETAINED_CONTEXT_LIFECYCLE
                | GRAPH_AUTHORITY_LIFECYCLE
                | crate::ROSTER_SOURCE_ISSUANCE_LIFECYCLE
                | crate::RESULT_SCHOOL_CORRESPONDENCE_LIFECYCLE
                | crate::RESULT_CALENDAR_CORRESPONDENCE_LIFECYCLE
        )
    )
}

pub(crate) fn is_graph_qualification_lifecycle(revision: Option<u32>) -> bool {
    is_qualification_lifecycle(revision)
        && !matches!(revision, Some(LIFECYCLE | RETAINED_CONTEXT_LIFECYCLE))
}

pub(crate) fn refuse_reserved(value: &str) -> crate::StoreResult<()> {
    if is_reserved_operation(value.as_bytes()) {
        return Err(PublicIdentityError::ReservedWrite.into());
    }
    Ok(())
}

pub(crate) fn is_reserved_operation(value: &[u8]) -> bool {
    value.starts_with(b"public_identity") || commit::source_batch::is_reserved_operation(value)
}

pub(crate) fn refuse_identity_write(
    store: &crate::Store,
    table: crate::Table,
) -> crate::StoreResult<()> {
    if table == crate::Table::AthleteIdentityDecisions
        && is_qualification_lifecycle(store.store_format()?.lifecycle_revision)
    {
        return Err(PublicIdentityError::ReservedWrite.into());
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicIdentityAdmissionResource {
    Bytes,
    Roots,
    Reservations,
}

#[derive(Debug, thiserror::Error)]
pub enum PublicIdentityError {
    #[error("public identity qualification requires a new empty root")]
    ExistingRoot,
    #[error("public identity qualification requires its explicit opener")]
    OptInRequired,
    #[error("public identity qualification requires {expected:?}, found {found:?}")]
    QualificationVersionMismatch {
        expected: census_domain::public_identity_qualification::PublicIdentityQualificationVersion,
        found: census_domain::public_identity_qualification::PublicIdentityQualificationVersion,
    },
    #[error("public identity capability belongs to another store")]
    ForeignStore,
    #[error("public identity ledger is missing or inconsistent: {0}")]
    Corrupt(&'static str),
    #[error(
        "public identity admission capacity exhausted for {resource:?}: needed {needed}, available {available}"
    )]
    AdmissionCapacity {
        resource: PublicIdentityAdmissionResource,
        needed: u64,
        available: u64,
    },
    #[error("public identity registry head moved")]
    HeadMoved,
    #[error("public identity operation conflicts with its immutable receipt")]
    OperationConflict,
    #[error("candidate cannot be established from retained source evidence")]
    NotSourceBound,
    #[error("retained review does not authorize this identity operation")]
    NotReviewed,
    #[error("reviewed identity request must use canonical ledger order")]
    NonCanonicalRequest,
    #[error("candidate already has a public identity binding")]
    AlreadyBound,
    #[error("public identity reserved namespace requires its owned writer")]
    ReservedWrite,
    #[error(transparent)]
    History(#[from] census_domain::public_identity_history::HistoryError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityCommitOutcomeV1 {
    public_id: PublicAthleteId,
    head: RegistryHead,
    replayed: bool,
}

impl IdentityCommitOutcomeV1 {
    pub fn public_id(&self) -> &PublicAthleteId {
        &self.public_id
    }
    pub fn head(&self) -> &RegistryHead {
        &self.head
    }
    pub fn replayed(&self) -> bool {
        self.replayed
    }
}

pub(crate) use local_adjudication::admission::Permit as ManualContextPermit;
