use census_crawl::registry::{descriptors, SourceDescriptor};
use census_domain::model::{
    serialized_digest, ContractRefV1, InstallationCapabilityV1, ServiceResourceProfileV1,
};
use serde::Serialize;

use super::InstallationCompileError;

const MAX_SOURCE_DESCRIPTORS: usize = 128;

#[derive(Debug, Clone, Serialize)]
pub struct SourceRegistryDescriptorV1 {
    pub version: u32,
    pub revision: u32,
    pub sources: Vec<&'static SourceDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallationDefaultsV1 {
    pub version: u32,
    pub revision: u32,
    pub name: &'static str,
    pub service_resources: ServiceResourceProfileV1,
    pub capability: InstallationCapabilityV1,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstallationContractsV1 {
    pub source_registry: SourceRegistryDescriptorV1,
    pub source_registry_reference: ContractRefV1,
    pub operating_defaults: InstallationDefaultsV1,
    pub operating_defaults_reference: ContractRefV1,
}

pub fn defaults() -> Result<InstallationContractsV1, InstallationCompileError> {
    let source_registry = source_catalog()?;
    let operating_defaults = InstallationDefaultsV1 {
        version: 1,
        revision: 1,
        name: "installation_enrollment_defaults_v1",
        service_resources: ServiceResourceProfileV1::ENROLLMENT_DEFAULTS_V1,
        capability: InstallationCapabilityV1::EnrollmentOnlyV1,
    };
    let source_registry_reference =
        ContractRefV1::new(1, 1, &serialized_digest(&source_registry)?)?;
    let operating_defaults_reference =
        ContractRefV1::new(1, 1, &serialized_digest(&operating_defaults)?)?;
    Ok(InstallationContractsV1 {
        source_registry,
        source_registry_reference,
        operating_defaults,
        operating_defaults_reference,
    })
}

fn source_catalog() -> Result<SourceRegistryDescriptorV1, InstallationCompileError> {
    let mut sources = Vec::new();
    sources.try_reserve_exact(MAX_SOURCE_DESCRIPTORS)?;
    for descriptor in descriptors().take(MAX_SOURCE_DESCRIPTORS + 1) {
        if sources.len() == MAX_SOURCE_DESCRIPTORS {
            return Err(InstallationCompileError::SourceCatalogBudget {
                max: MAX_SOURCE_DESCRIPTORS,
            });
        }
        sources.push(descriptor);
    }
    sources.sort_by_key(|descriptor| descriptor.slug);
    Ok(SourceRegistryDescriptorV1 {
        version: 1,
        revision: 1,
        sources,
    })
}
