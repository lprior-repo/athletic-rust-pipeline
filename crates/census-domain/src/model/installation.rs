use super::{CanonicalJsonError, ProfileError};

mod config;
mod enrollment;
mod resources;

pub use config::{InstallationCapabilityV1, InstallationConfigId, InstallationConfigV1};
pub use enrollment::InstallationEnrollmentV1;
pub use resources::ServiceResourceProfileV1;

#[derive(Debug, thiserror::Error)]
pub enum InstallationDomainError {
    #[error("unsupported {what} version {found}")]
    UnsupportedVersion { what: &'static str, found: u32 },
    #[error("service resource {field} is outside its existing supported envelope")]
    InvalidServiceResource { field: &'static str },
    #[error(transparent)]
    Profile(#[from] ProfileError),
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
}

pub(crate) fn require_version(
    what: &'static str,
    found: u32,
) -> Result<(), InstallationDomainError> {
    if found != 1 {
        return Err(InstallationDomainError::UnsupportedVersion { what, found });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
