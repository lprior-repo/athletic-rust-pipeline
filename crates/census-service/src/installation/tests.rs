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

#[test]
fn enrollment_defaults_preserve_exact_historical_bytes_digest_and_runtime_values() -> TestResult {
    let known = defaults()?;
    let expected = br#"{"version":1,"revision":1,"name":"installation_enrollment_defaults_v1","service_resources":{"max_concurrent":8,"memory_budget_bytes":51539607552,"drain_timeout_seconds":30},"capability":"enrollment_only_v1"}"#;
    assert_eq!(
        census_domain::model::serialized_bytes(&known.operating_defaults)?.as_slice(),
        expected
    );
    assert_eq!(
        serde_json::to_vec(&known.operating_defaults)?.as_slice(),
        expected
    );
    let digest = "d1424eef806c13d898f44542b0a4b119dbdbe9b55e2a246a7ca5e0ff1d987a84";
    assert_eq!(serialized_digest(&known.operating_defaults)?, digest);
    assert_eq!(
        known.operating_defaults_reference,
        ContractRefV1::new(1, 1, digest)?
    );
    let runtime = crate::bootstrap::ServeOptions::default();
    let parsed = crate::bootstrap::ServeOptions::from_env(std::iter::empty())?;
    for options in [runtime, parsed] {
        assert_eq!(options.max_concurrent, 8);
        assert_eq!(options.memory_budget_bytes, 51_539_607_552);
        assert_eq!(options.drain_timeout, std::time::Duration::from_secs(30));
        assert_eq!(
            options.max_concurrent,
            known.operating_defaults.service_resources.max_concurrent()
        );
        assert_eq!(
            options.memory_budget_bytes,
            known
                .operating_defaults
                .service_resources
                .memory_budget_bytes()
        );
        assert_eq!(
            options.drain_timeout.as_secs(),
            known
                .operating_defaults
                .service_resources
                .drain_timeout_seconds()
        );
    }
    assert_eq!(crate::bootstrap::DEFAULT_MAX_CONCURRENT, 8);
    assert_eq!(
        crate::bootstrap::DEFAULT_MEMORY_BUDGET_BYTES,
        51_539_607_552
    );
    assert_eq!(
        crate::bootstrap::DEFAULT_DRAIN_TIMEOUT,
        std::time::Duration::from_secs(30)
    );
    Ok(())
}
