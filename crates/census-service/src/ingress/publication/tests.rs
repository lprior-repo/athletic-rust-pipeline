use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn managed_publication_requires_explicit_endpoint_support() -> TestResult {
    require_managed(&PublicationCapabilities::current())?;
    for publication_contracts in [vec![], vec![PublicationContract::LegacySeparateV1]] {
        let error = require_managed(&PublicationCapabilities {
            publication_contracts,
        })
        .err()
        .ok_or("unsupported endpoint accepted")?;
        check!(eq; error.to_string(), "endpoint does not advertise managed_bundle_v2; no invocation was submitted");
    }
    Ok(())
}
