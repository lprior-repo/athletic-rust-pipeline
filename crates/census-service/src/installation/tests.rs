use super::*;
use census_domain::model::{
    serialized_digest, ContractRefV1, InstallationCapabilityV1, InstallationConfigV1, ProfileId,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn defaults_name_real_source_descriptors_exact_references_and_enrollment_only_capability(
) -> TestResult {
    let known = defaults()?;
    if known.source_registry_reference.digest() != serialized_digest(&known.source_registry)? {
        return Err("source descriptor digest mismatch".into());
    }
    if known.operating_defaults_reference.digest() != serialized_digest(&known.operating_defaults)?
    {
        return Err("defaults descriptor digest mismatch".into());
    }
    if known.operating_defaults.service_resources.max_concurrent() != 8
        || known
            .operating_defaults
            .service_resources
            .memory_budget_bytes()
            != crate::bootstrap::DEFAULT_MEMORY_BUDGET_BYTES
        || known
            .operating_defaults
            .service_resources
            .drain_timeout_seconds()
            != 30
    {
        return Err("default service resource mismatch".into());
    }
    if known.operating_defaults.capability != InstallationCapabilityV1::EnrollmentOnlyV1 {
        return Err("defaults suggest unsupported execution".into());
    }
    let mut real: Vec<_> = census_crawl::registry::descriptors()
        .map(|entry| entry.slug)
        .collect();
    real.sort();
    let actual: Vec<_> = known
        .source_registry
        .sources
        .iter()
        .map(|entry| entry.slug)
        .collect();
    if real != actual {
        return Err("source catalog is not the actual sorted registry".into());
    }
    Ok(())
}

#[test]
fn installation_compiler_refuses_changed_exact_reference_without_claiming_runtime_support(
) -> TestResult {
    let known = defaults()?;
    let id = ProfileId::parse(&"a".repeat(64))?;
    let valid = InstallationConfigV1::new(
        id.clone(),
        known.operating_defaults.service_resources,
        known.source_registry_reference.clone(),
        known.operating_defaults_reference.clone(),
        InstallationCapabilityV1::EnrollmentOnlyV1,
    );
    check_configuration(&valid)?;
    let changed = ContractRefV1::new(1, 2, &"b".repeat(64))?;
    for role in [
        InstallationContractRoleV1::SourceRegistry,
        InstallationContractRoleV1::OperatingDefaults,
    ] {
        let (source, operating) = match role {
            InstallationContractRoleV1::SourceRegistry => {
                (changed.clone(), known.operating_defaults_reference.clone())
            }
            InstallationContractRoleV1::OperatingDefaults => {
                (known.source_registry_reference.clone(), changed.clone())
            }
        };
        let config = InstallationConfigV1::new(
            id.clone(),
            valid.service_resources(),
            source,
            operating,
            valid.capability(),
        );
        if !matches!(check_configuration(&config), Err(InstallationCompileError::UnsupportedExactReference { role: found }) if found == role)
        {
            return Err("changed exact reference accepted".into());
        }
    }
    Ok(())
}
