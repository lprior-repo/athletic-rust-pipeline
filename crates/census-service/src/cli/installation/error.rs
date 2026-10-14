use census_service::installation::InstallationCompileError;
use census_store::{InstallationError, StoreError};

use crate::cli::profile::error::ProfileCliError;

pub(super) type InstallationCliResult<T> = Result<T, InstallationCliError>;

#[derive(Debug, thiserror::Error)]
pub(super) enum InstallationCliError {
    #[error("installation enroll/show require explicit --store and a stopped census-serve owner")]
    ExplicitStoreRequired,
    #[error(transparent)]
    Input(#[from] ProfileCliError),
    #[error(transparent)]
    Configuration(#[from] InstallationCompileError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("installation JSON output failed: {0}")]
    OutputJson(#[source] serde_json::Error),
    #[error("installation output could not be completed: {0}")]
    OutputIo(#[source] std::io::Error),
}

impl InstallationCliError {
    pub(super) fn output_failed(&self) -> bool {
        matches!(self, Self::OutputJson(_) | Self::OutputIo(_))
    }

    pub(super) fn code(&self) -> &'static str {
        match self {
            Self::ExplicitStoreRequired => "explicit_store_required",
            Self::Input(error) => error.code(),
            Self::Configuration(InstallationCompileError::UnsupportedExactReference { .. }) => {
                "installation_reference_unsupported"
            }
            Self::Configuration(InstallationCompileError::Store(error)) | Self::Store(error) => {
                store_code(error)
            }
            Self::Configuration(_) => "installation_configuration_failed",
            Self::OutputJson(_) | Self::OutputIo(_) => "output_failed",
        }
    }
}

fn store_code(error: &StoreError) -> &'static str {
    match error {
        StoreError::Installation(error) => match error {
            InstallationError::MarkerMismatch { .. } => "installation_marker_mismatch",
            InstallationError::MissingEnrollment => "installation_enrollment_missing",
            InstallationError::MissingConfig => "installation_config_missing",
            InstallationError::MultipleRecords => "installation_multiple_records",
            InstallationError::UnsupportedNamespace => "installation_namespace_unsupported",
            InstallationError::ConfigIdMismatch => "installation_config_id_mismatch",
            InstallationError::ForeignStore => "installation_foreign_store",
            InstallationError::ImmutableConflict => "installation_immutable_conflict",
            InstallationError::HistoricalStatePresent { .. } => {
                "installation_historical_state_present"
            }
            InstallationError::MetadataBudgetExceeded { .. } => {
                "installation_metadata_budget_exceeded"
            }
            InstallationError::UnsupportedVersion { .. } => "installation_version_unsupported",
            InstallationError::EnrollmentOnlyUnsupported { .. } => {
                "installation_execution_unsupported"
            }
            InstallationError::ProfileExecution(_) => {
                "installation_profile_configuration_unsupported"
            }
            InstallationError::Domain(_) => "installation_configuration_invalid",
        },
        StoreError::ProfileRegistry(_) => "installation_profile_invalid",
        StoreError::Open { .. } => "store_open_failed",
        StoreError::Read { .. } => "store_read_failed",
        StoreError::Write { .. } => "store_write_failed",
        StoreError::Io { .. } => "store_io_failed",
        StoreError::FormatNewer { .. } => "store_schema_newer",
        StoreError::MigrationRequired { .. } => "store_migration_required",
        StoreError::SchemaUnknown { .. } => "store_schema_invalid",
        StoreError::RowTooLarge { .. } => "installation_record_oversized",
        StoreError::Decode { .. } | StoreError::Json { .. } => "installation_record_invalid",
        _ => "store_failed",
    }
}
