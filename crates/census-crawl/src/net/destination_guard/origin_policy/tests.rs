use super::*;

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn empty_grants_allow_only_the_original_scheme_hostname_and_effective_port() -> TestResult {
    let original = Url::parse("https://SOURCE.example/start/")?;
    for (destination, allowed) in [
        ("https://source.example/start", true),
        ("https://source.example:443/destination?query=1", true),
        ("https://source.example:444/destination", false),
        ("http://source.example:443/destination", false),
        ("http://source.example/destination", false),
        ("https://other.example/destination", false),
        ("https://sub.source.example/destination", false),
        ("file:///destination", false),
    ] {
        check!(eq;
            permits_redirect(&original, &Url::parse(destination)?, &[]),
            allowed,
            "{destination}"
        );
    }
    let original = Url::parse("http://source.example/start")?;
    check!(permits_redirect(
        &original,
        &Url::parse("http://source.example:80/destination")?,
        &[]
    ));
    Ok(())
}

#[test]
fn explicit_grants_allow_origin_changes_only_for_exact_hosts_and_dot_bounded_subdomains(
) -> TestResult {
    let original = Url::parse("https://original.example/start")?;
    let grants = vec!["source.example".to_string()];
    for (destination, allowed) in [
        ("http://source.example/destination", true),
        ("https://source.example:444/destination", true),
        ("http://sub.source.example:8080/destination", true),
        ("http://notsource.example/destination", false),
        ("http://source.example.other/destination", false),
        ("http://other.example/destination", false),
    ] {
        check!(eq;
            permits_redirect(&original, &Url::parse(destination)?, &grants),
            allowed,
            "{destination}"
        );
    }
    check!(host_granted("SUB.SOURCE.EXAMPLE", &grants));
    check!(!host_granted("notsource.example", &grants));
    Ok(())
}

#[test]
fn intermediate_granted_origin_does_not_become_the_original_origin() -> TestResult {
    let original = Url::parse("https://original.example/start")?;
    let intermediate = Url::parse("http://granted.example:8080/hop")?;
    let ungranted = Url::parse("http://original.example:8080/destination")?;
    let grants = vec!["granted.example".to_string()];
    check!(permits_redirect(&original, &intermediate, &grants));
    check!(!permits_redirect(&original, &ungranted, &grants));
    Ok(())
}
