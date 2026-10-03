use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn literal_destinations_reject_special_use_addresses_and_credentials() {
    let guard = DestinationGuard::new(Vec::new());
    for url in [
        "http://127.0.0.1/",
        "http://2130706433/",
        "http://10.0.0.1/",
        "http://172.16.0.1/",
        "http://192.168.0.1/",
        "http://169.254.169.254/",
        "http://0.0.0.0/",
        "http://100.64.0.1/",
        "http://224.0.0.1/",
        "http://[::1]/",
        "http://[::ffff:127.0.0.1]/",
        "http://[fc00::1]/",
        "http://[fe80::1]/",
        "http://[2002:7f00:1::]/",
        "https://user:password@example.com/",
        "file:///etc/passwd",
    ] {
        assert!(guard.validate_url(url).is_err(), "{url}");
    }
    assert!(guard
        .validate_url("https://[2001:4860:4860::8888]/")
        .is_ok());
    assert!(guard.validate_url("https://example.com/").is_ok());
}

#[test]
fn authorized_domains_do_not_authorize_private_dns_answers() -> TestResult {
    let guard = DestinationGuard::new(vec!["example.com".to_string()]);
    check!(guard.is_authorized("sub.example.com"));
    check!(guard
        .validate_ip("sub.example.com", "127.0.0.1".parse()?)
        .is_err());
    check!(guard
        .validate_ip("example.com", "10.0.0.1".parse()?)
        .is_err());
    check!(guard
        .validate_ip("example.com", "93.184.216.34".parse()?)
        .is_ok());
    let local = DestinationGuard::new(vec!["127.0.0.1".to_string()]);
    check!(local.validate_url("http://127.0.0.1:8080/").is_ok());
    check!(local
        .validate_ip("example.com", "127.0.0.1".parse()?)
        .is_err());
    Ok(())
}

#[test]
fn public_ipv4_neighbors_are_not_reserved_networks() -> TestResult {
    let guard = DestinationGuard::new(Vec::new());
    for ip in [
        "192.0.78.24",
        "192.0.1.1",
        "192.1.0.1",
        "198.51.99.1",
        "203.0.112.1",
    ] {
        check!(
            guard.validate_ip("public.example", ip.parse()?).is_ok(),
            "{ip}"
        );
    }
    for ip in [
        "192.0.0.1",
        "192.0.2.1",
        "192.168.1.1",
        "198.51.100.1",
        "203.0.113.1",
    ] {
        check!(
            guard.validate_ip("public.example", ip.parse()?).is_err(),
            "{ip}"
        );
    }
    Ok(())
}

#[test]
fn denied_local_request_issues_neither_robots_nor_payload() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
            let root = tempfile::tempdir()?;
            let fetcher = crate::net::Fetcher::new(
                root.path(),
                None,
                std::time::Duration::ZERO,
                std::collections::HashMap::new(),
                Vec::new(),
            )?;
            let url = format!("http://{}/evidence", listener.local_addr()?);
            let result = fetcher
                .get(&url, &crate::net::FetchOptions::default())
                .await;
            check!(matches!(result, Err(FetchError::Policy { .. })));
            check!(
                tokio::time::timeout(std::time::Duration::from_millis(20), listener.accept())
                    .await
                    .is_err()
            );
            Ok(())
        })
}
