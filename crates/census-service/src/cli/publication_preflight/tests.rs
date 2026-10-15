use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn managed_publication_requires_explicit_endpoint_support() -> TestResult {
    require_managed(&PublicationCapabilities::current())?;
    for publication_contracts in [vec![], vec![PublicationContract::LegacySeparateV1]] {
        require_managed(&PublicationCapabilities {
            publication_contracts,
        })
        .err()
        .ok_or("unsupported endpoint accepted")?;
    }
    Ok(())
}
