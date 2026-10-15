use census_domain::public_identity_history::{PublicAthleteId, RegistryHead};

mod authority;
mod codec;
mod commit;
mod init;
mod inventory;
pub(crate) mod qualification;
mod witness;

pub(crate) const LIFECYCLE: u32 =
    census_domain::public_identity_qualification::PUBLIC_IDENTITY_QUALIFICATION_LIFECYCLE;
pub(crate) const OPERATION_PREFIX: &str = "public_identity_v1:";
const DECLARATION: &str = "public_identity:v1:qualification";
const HEAD: &str = "public_identity:v1:head";
const HISTORY: &str = "public_identity:v1:history";
pub const MAX_PUBLIC_IDENTITY_HISTORY_BYTES: usize = 16 * 1024 * 1024;

pub(crate) use qualification::{preflight_keyspaces, require_legacy, validate};
pub use qualification::{PublicIdentityQualificationStore, PublicIdentityWriterV1};

pub(crate) fn refuse_reserved(value: &str) -> crate::StoreResult<()> {
    if value.starts_with("public_identity") {
        return Err(PublicIdentityError::ReservedWrite.into());
    }
    Ok(())
}

pub(crate) fn refuse_identity_write(
    store: &crate::Store,
    table: crate::Table,
) -> crate::StoreResult<()> {
    if table == crate::Table::AthleteIdentityDecisions
        && store.store_format()?.lifecycle_revision == Some(LIFECYCLE)
    {
        return Err(PublicIdentityError::ReservedWrite.into());
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum PublicIdentityError {
    #[error("public identity qualification requires a new empty root")]
    ExistingRoot,
    #[error("public identity qualification requires its explicit opener")]
    OptInRequired,
    #[error("public identity capability belongs to another store")]
    ForeignStore,
    #[error("public identity ledger is missing or inconsistent: {0}")]
    Corrupt(&'static str),
    #[error("public identity registry head moved")]
    HeadMoved,
    #[error("public identity operation conflicts with its immutable receipt")]
    OperationConflict,
    #[error("candidate cannot be established from retained source evidence")]
    NotSourceBound,
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
