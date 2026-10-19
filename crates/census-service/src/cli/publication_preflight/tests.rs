use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn selected_publication_requires_the_exact_endpoint_contract() -> TestResult {
    require_contract(
        &PublicationCapabilities::current(),
        PublicationContract::SelectedPublicationV1,
    )?;
    require_contract(
        &PublicationCapabilities {
            publication_contracts: vec![PublicationContract::ManagedBundleV2],
            national_generation_revision: None,
        },
        PublicationContract::SelectedPublicationV1,
    )
    .err()
    .ok_or("managed-only endpoint accepted selected publication")?;
    Ok(())
}
