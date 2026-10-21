use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn selected_publication_requires_the_exact_endpoint_contract() -> TestResult {
    require_contract(
        &PublicationCapabilities::current(),
        PublicationContract::SelectedPublicationV2,
    )?;
    require_contract(
        &PublicationCapabilities {
            publication_contracts: vec![],
            national_generation_revision: None,
        },
        PublicationContract::SelectedPublicationV2,
    )
    .err()
    .ok_or("empty endpoint accepted selected publication")?;
    Ok(())
}
