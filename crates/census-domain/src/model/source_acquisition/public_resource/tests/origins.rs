use super::*;

#[test]
fn calendar_and_robots_targets_keep_exact_origin_and_refuse_auth_queries() -> TestResult {
    let calendar = PublicSourceResourceV1::school_calendar(
        "12684",
        "https://www.example.org",
        Sport::CrossCountry,
        year()?,
    )?;
    calendar.validate_target(
        UsJurisdiction::Michigan,
        "https://www.example.org/athletics/calendar/2026-27",
    )?;
    for tail in [
        "/Login",
        "/account/calendar",
        "/oauth2/authorize",
        "/calendar?year=2026",
        "/calendar?redirect_uri=https://evil.org",
        "/%6cogin",
        "/calendar/../calendar",
        "/calendar#login",
        "/admin/schedule",
        "/sign-in/sports",
        "/calendar\\login",
        "/signout",
    ] {
        check!(calendar
            .validate_target(
                UsJurisdiction::Michigan,
                &format!("https://www.example.org{tail}")
            )
            .is_err());
    }
    check!(calendar
        .validate_target(
            UsJurisdiction::Michigan,
            "https://other.example.org/calendar"
        )
        .is_err());
    let robots = PublicSourceResourceV1::robots("https://www.example.org")?;
    robots.validate_target(
        UsJurisdiction::Michigan,
        "https://www.example.org/robots.txt",
    )?;
    for tail in [
        "/Robots.txt",
        "/robots.txt/",
        "/robots.txt?",
        "/ROBOTs.txt",
        "/robots.txt/extra",
        "/%72obots.txt",
    ] {
        check!(robots
            .validate_target(
                UsJurisdiction::Michigan,
                &format!("https://www.example.org{tail}")
            )
            .is_err());
    }
    check!(robots
        .validate_target(
            UsJurisdiction::Michigan,
            "https://other.example.org/robots.txt"
        )
        .is_err());
    Ok(())
}

#[test]
fn only_canonical_public_dns_origins_enter_either_resource_kind() -> TestResult {
    for origin in [
        "http://www.example.org",
        "https://www.example.org/",
        "https://www.example.org/path",
        "https://www.example.org?",
        "https://www.example.org#",
        "https://user@www.example.org",
        "https://www.example.org:443",
        "https://www.example.org:444",
        "https://WWW.EXAMPLE.ORG",
        "https://127.0.0.1",
        "https://2130706433",
        "https://10.1.2.3",
        "https://192.168.1.1",
        "https://169.254.169.254",
        "https://[::1]",
        "https://[fc00::1]",
        "https://8.8.8.8",
        "https://localhost",
        "https://school.localhost",
        "https://school.local",
        "https://school.internal",
        "https://school.lan",
        "https://school.corp",
        "https://school.home.arpa",
        "https://school.test",
        "https://athletic.net",
        "https://www.athletic.net",
        "https://live.athletic.net",
        "https://example.org.",
        "https://-school.example.org",
        "https://school-.example.org",
    ] {
        check!(PublicSourceResourceV1::robots(origin).is_err());
        check!(PublicSourceResourceV1::school_calendar(
            "12684",
            origin,
            Sport::CrossCountry,
            year()?
        )
        .is_err());
    }
    let retained = PublicSourceResourceV1::robots("https://athletic.net.example.org")?;
    check!(eq; retained.origin(), Some("https://athletic.net.example.org"));
    Ok(())
}

#[test]
fn dns_host_and_label_limits_apply_at_the_exact_boundary() -> TestResult {
    let labels = [
        "a".repeat(63),
        "b".repeat(63),
        "c".repeat(63),
        "d".repeat(61),
    ];
    let host = labels.join(".");
    check!(eq; host.len(), 253);
    let origin = format!("https://{host}");
    let resource = PublicSourceResourceV1::robots(&origin)?;
    resource.validate_target(UsJurisdiction::Michigan, &format!("{origin}/robots.txt"))?;
    check!(PublicSourceResourceV1::robots(&format!("https://{host}e")).is_err());
    check!(
        PublicSourceResourceV1::robots(&format!("https://{}.example.org", "a".repeat(64))).is_err()
    );
    Ok(())
}
