use census_domain::model::{InstallationConfigV1, InstallationDomainError};
use census_store::{InstallationControl, InstallationEnrollmentOutcome, StoreError};

mod contracts;

pub use contracts::{
    defaults, InstallationContractsV1, InstallationDefaultsV1, SourceRegistryDescriptorV1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallationContractRoleV1 {
    SourceRegistry,
    OperatingDefaults,
}

#[derive(Debug, thiserror::Error)]
pub enum InstallationCompileError {
    #[error("unsupported exact installation reference for {role:?}")]
    UnsupportedExactReference { role: InstallationContractRoleV1 },
    #[error("source catalog exceeds its {max}-descriptor budget")]
    SourceCatalogBudget { max: usize },
    #[error("bounded source catalog allocation failed: {0}")]
    Allocation(#[from] std::collections::TryReserveError),
    #[error(transparent)]
    Domain(#[from] InstallationDomainError),
    #[error(transparent)]
    Canonical(#[from] census_domain::model::CanonicalJsonError),
    #[error(transparent)]
    Reference(#[from] census_domain::model::ProfileError),
    #[error(transparent)]
    Store(#[from] StoreError),
}

pub fn check_configuration(config: &InstallationConfigV1) -> Result<(), InstallationCompileError> {
    let known = defaults()?;
    if config.source_registry() != &known.source_registry_reference {
        return Err(InstallationCompileError::UnsupportedExactReference {
            role: InstallationContractRoleV1::SourceRegistry,
        });
    }
    if config.operating_defaults() != &known.operating_defaults_reference {
        return Err(InstallationCompileError::UnsupportedExactReference {
            role: InstallationContractRoleV1::OperatingDefaults,
        });
    }
    Ok(())
}

pub fn enroll(
    control: &mut InstallationControl,
    config: &InstallationConfigV1,
) -> Result<InstallationEnrollmentOutcome, InstallationCompileError> {
    check_configuration(config)?;
    control.enroll(config).map_err(Into::into)
}

#[cfg(test)]
mod tests;
