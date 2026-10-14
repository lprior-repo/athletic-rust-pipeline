use census_domain::model::{InstallationConfigV1, InstallationEnrollmentV1};
use serde::Serialize;

mod control;
mod eligibility;
mod preservation;
mod read;
pub(crate) use preservation::validate_backup;

pub use control::InstallationControl;
pub(crate) use read::{require_legacy_execution, validate_format};

pub const MAX_INSTALLATION_RECORD_BYTES: usize = crate::records::codec::MAX_RECORD_BYTES;
pub(crate) const ENROLLMENT_KEY: &str = "installation:v1:enrollment";
pub(crate) const CONFIG_PREFIX: &str = "installation:v1:config:";
pub(crate) const PRODUCTION_LIFECYCLE: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallationState {
    NotEnrolled,
    Enrolled {
        enrollment: InstallationEnrollmentV1,
        config: InstallationConfigV1,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallationEnrollmentOutcome {
    Enrolled(InstallationEnrollmentV1),
    Repeated(InstallationEnrollmentV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoricalStateKind {
    Entities,
    Receipts,
    Journal,
    Metadata,
    Generations,
    LegacyRun,
    SharedOwner,
    Artifacts,
}

#[derive(Debug, thiserror::Error)]
pub enum InstallationError {
    #[error("installation records do not match lifecycle marker {found:?}")]
    MarkerMismatch { found: Option<u32> },
    #[error("installation metadata uses an unsupported namespace")]
    UnsupportedNamespace,
    #[error("installation enrollment is missing")]
    MissingEnrollment,
    #[error("installation config is missing")]
    MissingConfig,
    #[error("installation metadata contains more than its immutable enrollment/config pair")]
    MultipleRecords,
    #[error("installation config identifier does not match retained bytes")]
    ConfigIdMismatch,
    #[error("installation enrollment belongs to a foreign store identity")]
    ForeignStore,
    #[error("installation enrollment conflicts with its immutable configuration")]
    ImmutableConflict,
    #[error("installation enrollment refuses existing {kind:?} state")]
    HistoricalStatePresent { kind: HistoricalStateKind },
    #[error("installation metadata exceeds its {max}-row verification budget")]
    MetadataBudgetExceeded { max: usize },
    #[error("unsupported {what} version {found}")]
    UnsupportedVersion { what: &'static str, found: u32 },
    #[error("enrolled installation {identity} with config {config_id} supports enrollment only; production execution remains unsupported", identity = .store_identity.as_str())]
    EnrollmentOnlyUnsupported {
        store_identity: census_domain::model::StoreIdentity,
        config_id: census_domain::model::InstallationConfigId,
    },
    #[error(transparent)]
    ProfileExecution(#[from] census_domain::profile_execution::ProfileExecutionError),
    #[error(transparent)]
    Domain(#[from] census_domain::model::InstallationDomainError),
}
