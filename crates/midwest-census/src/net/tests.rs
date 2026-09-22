use super::*;

/// Build a fetcher whose cache lives in a throwaway directory. Authorization is the only
/// property under test here, so no request is ever issued.
fn fetcher_with(authorized: Vec<String>) -> (Fetcher, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        Duration::from_millis(1),
        HashMap::new(),
        authorized,
    )
    .expect("fetcher");
    (fetcher, dir)
}

#[test]
fn no_host_is_authorized_by_default() {
    let (fetcher, _dir) = fetcher_with(Vec::new());
    assert!(!fetcher.is_authorized_host("www.athletic.net"));
    assert!(!fetcher.is_authorized_host("milesplit.com"));
}

#[test]
fn a_bare_domain_authorizes_its_subdomains_but_not_lookalikes() {
    let (fetcher, _dir) = fetcher_with(vec!["athletic.net".to_string()]);
    assert!(fetcher.is_authorized_host("athletic.net"));
    assert!(fetcher.is_authorized_host("www.athletic.net"));
    assert!(fetcher.is_authorized_host("WWW.Athletic.NET"));
    // Suffix matching must not leak to a domain that merely ends with the same text.
    assert!(!fetcher.is_authorized_host("notathletic.net"));
    assert!(!fetcher.is_authorized_host("athletic.net.evil.com"));
}

#[test]
fn an_exact_host_never_widens_into_its_parent_domain() {
    let (fetcher, _dir) = fetcher_with(vec!["www.example.com".to_string()]);
    assert!(fetcher.is_authorized_host("www.example.com"));
    assert!(fetcher.is_authorized_host("cdn.www.example.com"));
    assert!(!fetcher.is_authorized_host("example.com"));
    assert!(!fetcher.is_authorized_host("other.example.com"));
}

#[test]
fn authorization_never_relaxes_the_two_rps_ceiling() {
    let (fetcher, _dir) = fetcher_with(vec!["athletic.net".to_string()]);
    // The policy ceiling is a floor on spacing: even a 1 ms configured delay is raised.
    assert!(MIN_AUTHORIZED_DELAY >= Duration::from_millis(500));
    let (unauthorized, _dir2) = fetcher_with(Vec::new());
    assert!(!unauthorized.is_authorized_host("athletic.net"));
    assert!(fetcher.is_authorized_host("athletic.net"));
}

#[test]
fn robots_rules_honour_longest_match_and_allow_ties() {
    let rules = parse_robots(
        "User-agent: *\nDisallow: /rankings\nDisallow: /api/\nAllow: /api/public/\nCrawl-delay: 2\n",
    );
    assert!(!rules.allows("/rankings/leaders"));
    assert!(!rules.allows("/api/v1/meets"));
    assert!(rules.allows("/api/public/x"));
    assert!(rules.allows("/teams/1234/roster"));
    assert_eq!(rules.crawl_delay, Some(Duration::from_secs(2)));
}

#[test]
fn robots_absent_or_empty_allows_everything() {
    let empty = parse_robots("");
    assert!(empty.allows("/anything"));
    let no_group = parse_robots("Disallow: /x\n");
    assert!(no_group.allows("/x"), "rules outside a group do not apply");
}

#[test]
fn a_hostile_crawl_delay_cannot_panic_or_park_the_walk() {
    // `Duration::from_secs_f64` panics on these, and a fetched robots.txt is untrusted input: two
    // lines served by a crawled host must not be able to kill the walk that fetched them.
    for hostile in ["inf", "-inf", "nan", "1e30", "1e300", "-5"] {
        let body = format!("User-agent: *\nDisallow: /private\nCrawl-delay: {hostile}\n");
        let rules = parse_robots(&body);
        assert!(!rules.allows("/private/x"), "{hostile} must still disallow");
        assert!(
            rules
                .crawl_delay
                .is_none_or(|delay| delay <= std::time::Duration::from_secs(3600)),
            "{hostile} must be refused or clamped, got {:?}",
            rules.crawl_delay
        );
    }
    // A finite but absurd delay is clamped rather than honoured: the host is asking not to be
    // walked, and the run's own budget stays intact.
    let clamped = parse_robots("User-agent: *\nCrawl-delay: 1e9\n");
    assert_eq!(
        clamped.crawl_delay,
        Some(std::time::Duration::from_secs(3600))
    );
    // An ordinary delay is honoured unchanged.
    let honoured = parse_robots("User-agent: *\nCrawl-delay: 2\n");
    assert_eq!(
        honoured.crawl_delay,
        Some(std::time::Duration::from_secs(2))
    );
}

#[test]
fn robots_named_agent_groups_are_ignored() {
    let rules =
        parse_robots("User-agent: GPTBot\nDisallow: /\n\nUser-agent: *\nDisallow: /private\n");
    assert!(rules.allows("/teams"));
    assert!(!rules.allows("/private/x"));
}

#[test]
fn jittered_delay_increases_with_attempt() {
    let d1 = jittered_delay(1);
    let d2 = jittered_delay(2);
    let d3 = jittered_delay(3);
    // Jitter adds noise, so strict ordering isn't guaranteed.
    // But the expected value increases.
    assert!(d1 <= d2);
    assert!(d2 <= d3);
    assert!(d3 <= Duration::from_secs(10));
}

#[test]
fn jittered_delay_respects_cap() {
    for attempt in 1..=10 {
        let d = jittered_delay(attempt);
        assert!(
            d <= Duration::from_secs(10),
            "delay for attempt {attempt} exceeds 10s cap"
        );
    }
}

#[test]
fn cache_key_pins_the_on_disk_cache_layout() {
    // Cached bodies live at `{key}.body` / `{key}.meta.json`, so the key derivation is part
    // of the on-disk layout: `GET`-with-no-body and `POST`-with-a-body must keep the same
    // keys across refactors, or a re-run stops being free.
    assert_eq!(
        Fetcher::key_for("GET", "https://example.com/teams", ""),
        "2ee9e0985d9a4ffc8864d9dfaae08524"
    );
    assert_eq!(
        Fetcher::key_for("POST", "https://example.com/api", "q=1&page=2"),
        "fd3996c5f6d99f4badb15fb729c483c8"
    );
    // Two queries against one endpoint are two documents.
    assert_ne!(
        Fetcher::key_for("POST", "https://example.com/api", "q=1&page=2"),
        Fetcher::key_for("POST", "https://example.com/api", "q=1&page=3")
    );
}
