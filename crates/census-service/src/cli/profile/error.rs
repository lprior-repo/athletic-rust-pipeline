use census_domain::model::{ProfileError, ProfileSelectionError};
use census_store::{ProfileRegistryError, StoreError, StoreLineageError};
use std::path::PathBuf;

use super::input::InputKind;

pub(super) type ProfileCliResult<T> = Result<T, ProfileCliError>;

#[derive(Debug, thiserror::Error)]
pub(super) enum ProfileCliError {
    #[error("profile commands require explicit --store and a stopped census-serve owner")]
    StoreRequired,
    #[error("profile select requires exactly one of --id or --selection")]
    SelectionArguments,
    #[error("cannot inspect input file {path}: {source}")]
    InputMetadata {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("input {path} must be a regular file")]
    InputNotFile { path: PathBuf },
    #[error("cannot open input file {path}: {source}")]
    InputOpen {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("input read failed: {0}")]
    InputRead(#[source] std::io::Error),
    #[error("input holds {observed} bytes, exceeding the {max}-byte ceiling")]
    InputTooLarge { observed: u64, max: usize },
    #[error("bounded input allocation failed: {0}")]
    AllocationFailed(#[source] std::collections::TryReserveError),
    #[error("bounded input size arithmetic failed")]
    InputSizeArithmetic,
    #[error("bounded input reader exhausted its finite progress budget")]
    InputReadBudget,
    #[error("{kind} JSON is malformed or violates a checked field: {source}")]
    InputJson {
        kind: InputKind,
        #[source]
        source: serde_json::Error,
    },
    #[error("unsupported {kind} version {found}; supported version is 1")]
    InputVersion { kind: InputKind, found: u32 },
    #[error("invalid query profile: {0}")]
    InvalidProfile(#[source] ProfileError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("profile JSON output failed: {0}")]
    OutputJson(#[source] serde_json::Error),
    #[error("profile output could not be completed: {0}")]
    OutputIo(#[source] std::io::Error),
}

impl ProfileCliError {
    pub(super) fn output_failed(&self) -> bool {
        matches!(self, Self::OutputJson(_) | Self::OutputIo(_))
    }

    pub(super) fn code(&self) -> &'static str {
        match self {
            Self::StoreRequired => "explicit_store_required",
            Self::SelectionArguments => "selection_arguments_invalid",
            Self::InputMetadata { .. } => "input_metadata_failed",
            Self::InputNotFile { .. } => "input_not_regular_file",
            Self::InputOpen { .. } => "input_open_failed",
            Self::InputRead(_) => "input_read_failed",
            Self::InputTooLarge { .. } => "input_oversized",
            Self::AllocationFailed(_) => "input_allocation_failed",
            Self::InputSizeArithmetic => "input_size_arithmetic_failed",
            Self::InputReadBudget => "input_read_budget_exhausted",
            Self::InputJson { .. } => "input_json_invalid",
            Self::InputVersion { .. } => "input_version_unsupported",
            Self::InvalidProfile(_) => "profile_invalid",
            Self::Store(error) => store_code(error),
            Self::OutputJson(_) | Self::OutputIo(_) => "output_failed",
        }
    }
}

fn store_code(error: &StoreError) -> &'static str {
    match error {
        StoreError::ProfileRegistry(error) => registry_code(error),
        StoreError::StoreLineage(StoreLineageError::Missing) => "store_identity_missing",
        StoreError::StoreLineage(_) | StoreError::StoreIdentity(_) => "store_identity_invalid",
        StoreError::Open { .. } => "store_open_failed",
        StoreError::Read { .. } => "store_read_failed",
        StoreError::Write { .. } => "store_write_failed",
        StoreError::FormatNewer { .. } => "store_schema_newer",
        StoreError::MigrationRequired { .. } => "store_migration_required",
        StoreError::SchemaUnknown { .. } => "store_schema_invalid",
        StoreError::RowTooLarge { .. } => "profile_record_oversized",
        StoreError::CanonicalJson(_) => "canonical_encoding_failed",
        _ => "store_failed",
    }
}

fn registry_code(error: &ProfileRegistryError) -> &'static str {
    match error {
        ProfileRegistryError::MissingProfile { .. } => "profile_missing",
        ProfileRegistryError::Oversized { .. } => "retained_profile_oversized",
        ProfileRegistryError::Malformed { .. } => "retained_profile_malformed",
        ProfileRegistryError::UnsupportedVersion { .. } => "retained_profile_version_unsupported",
        ProfileRegistryError::IdMismatch { .. } => "retained_profile_id_mismatch",
        ProfileRegistryError::ImmutableConflict { .. } => "profile_immutable_conflict",
        ProfileRegistryError::InvalidPayload(_) => "retained_profile_invalid",
        ProfileRegistryError::Selection(ProfileSelectionError::ForeignStore) => {
            "selection_foreign_store"
        }
        ProfileRegistryError::Selection(ProfileSelectionError::UnsupportedVersion { .. }) => {
            "selection_version_unsupported"
        }
        ProfileRegistryError::Canonical(_) => "canonical_encoding_failed",
    }
}
