use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn managed_publication_requires_explicit_endpoint_support() -> TestResult {
    require_contract(
        &PublicationCapabilities::current(),
        PublicationContract::ManagedBundleV2,
    )?;
    for publication_contracts in [vec![], vec![PublicationContract::LegacySeparateV1]] {
        require_contract(
            &PublicationCapabilities {
                publication_contracts,
            },
            PublicationContract::ManagedBundleV2,
        )
        .err()
        .ok_or("unsupported endpoint accepted")?;
    }
    Ok(())
}

#[test]
fn selected_publication_requires_the_exact_endpoint_contract() -> TestResult {
    require_contract(
        &PublicationCapabilities::current(),
        PublicationContract::SelectedPublicationV1,
    )?;
    require_contract(
        &PublicationCapabilities {
            publication_contracts: vec![PublicationContract::ManagedBundleV2],
        },
        PublicationContract::SelectedPublicationV1,
    )
    .err()
    .ok_or("managed-only endpoint accepted selected publication")?;
    Ok(())
}
