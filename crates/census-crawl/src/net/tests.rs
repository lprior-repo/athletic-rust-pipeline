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
    let clamped = parse_robots("User-agent: *\nCrawl-delay: 1e9\n");
    assert_eq!(
        clamped.crawl_delay,
        Some(std::time::Duration::from_secs(3600))
    );
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
fn cache_key_pins_the_on_disk_cache_layout() {
    assert_eq!(
        Fetcher::key_for("GET", "https://example.com/teams", ""),
        "2ee9e0985d9a4ffc8864d9dfaae08524"
    );
    assert_eq!(
        Fetcher::key_for("POST", "https://example.com/api", "q=1&page=2"),
        "fd3996c5f6d99f4badb15fb729c483c8"
    );
    assert_ne!(
        Fetcher::key_for("POST", "https://example.com/api", "q=1&page=2"),
        Fetcher::key_for("POST", "https://example.com/api", "q=1&page=3")
    );
}

/// A body that does not match its metadata digest is a cache miss, never served as evidence.
#[tokio::test]
async fn a_corrupted_cache_body_is_rejected_not_served() {
    let dir = tempfile::tempdir().expect("temp dir");
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher");
    let key = Fetcher::key_for("GET", "https://example.com/teams", "");
    let (body_path, meta_path) = fetcher.cache_paths(&key);

    let body = b"valid body";
    use super::cache::{content_digest, write_cache};
    let meta = super::cache::CacheMeta {
        url: "https://example.com/teams".to_string(),
        method: "GET".to_string(),
        status: 200,
        key_prefix: "deadbeef".to_string(),
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: "2025-01-01T00:00:00Z".to_string(),
        etag: None,
        last_modified: None,
        content_type: None,
    };
    write_cache(&body_path, &meta_path, body, &meta).expect("write");

    let (cached_meta, cached_body) = super::cache::read_cache(&body_path, &meta_path)
        .expect("read")
        .expect("cache hit");
    assert_eq!(cached_body, body);
    assert_eq!(cached_meta.bytes, body.len());

    std::fs::write(&body_path, b"corrupted body!!!").expect("corrupt");

    let result = super::cache::read_cache(&body_path, &meta_path).expect("read");
    assert!(result.is_none(), "corrupted body must be a cache miss");
}

/// A robots.txt body that is completely empty (as might happen on a 5xx with no body) yields
/// `fetched: true` with no rules — the host is treated as having rules but none were parsed.
/// This is the "unknown" outcome: we fetched, we just don't know the rules.
#[test]
fn an_empty_robots_body_is_fetched_but_has_no_rules() {
    let rules = parse_robots("");
    assert!(
        rules.was_fetched(),
        "empty body is a fetched file, not an absent one"
    );
    assert!(rules.allows("/"));
}

/// A robots.txt body with only comments and whitespace is parsed as fetched with no rules.
#[test]
fn a_comment_only_robots_body_is_fetched_but_has_no_rules() {
    let rules = parse_robots("# just a comment\n  \n# nothing useful\n");
    assert!(rules.was_fetched(), "a comment-only body is still fetched");
}

/// §45: the physical count is derived from the other two, so a cached run cannot claim traffic.
#[test]
fn a_cache_hit_is_not_a_physical_request() {
    let stats = FetchStats {
        requests: 10,
        cache_hits: 7,
        ..FetchStats::default()
    };
    assert_eq!(stats.physical_requests(), 3);
    assert_eq!(stats.useful_records_per_physical_request(9), Some(3.0));
}

/// §45: records per request is absent when no request reached the origin, never infinite.
#[test]
fn the_efficiency_ratio_is_absent_without_physical_requests() {
    let stats = FetchStats::default();
    assert_eq!(stats.physical_requests(), 0);
    assert_eq!(stats.useful_records_per_physical_request(4_000), None);
}

/// §45: a percentile is the edge of the bucket that covers it, and the mean is exact.
#[test]
fn latency_percentiles_are_bucket_upper_bounds() {
    let mut stats = FetchStats::default();
    for _ in 0..98 {
        stats.record_latency(12);
    }
    stats.record_latency(700);
    stats.record_latency(30_000);

    assert_eq!(stats.latency_samples(), 100);
    assert_eq!(stats.latency_ms_max, 30_000);
    assert_eq!(stats.latency_ms_sum, 31_876);
    assert_eq!(stats.latency_ms_avg(), Some(318));
    assert_eq!(stats.latency_percentile_ms(50), Some(20));
    assert_eq!(stats.latency_percentile_ms(95), Some(20));
    assert_eq!(stats.latency_percentile_ms(99), Some(900));
    assert_eq!(stats.latency_percentile_ms(100), Some(30_000));
}

/// A percentile of no samples is absent rather than zero: nothing was measured.
#[test]
fn a_percentile_of_no_samples_is_absent() {
    let stats = FetchStats::default();
    assert_eq!(stats.latency_samples(), 0);
    assert_eq!(stats.latency_ms_avg(), None);
    assert_eq!(stats.latency_percentile_ms(99), None);
}

/// §45: a per-source row is the origin, so two pages of one host cannot become two rows — and a URL
/// the parser refuses is still counted, under its own text rather than dropped.
#[test]
fn a_source_row_is_the_origin_not_the_page() {
    assert_eq!(
        host_of("https://www.athletic.net/team/1/x?a=b"),
        "www.athletic.net"
    );
    assert_eq!(host_of("http://example.com"), "example.com");
    assert_eq!(host_of("http://example.com:8080/page"), "example.com");
    assert_eq!(host_of("not-a-url"), "not-a-url");
}

/// §45: a challenge is counted apart from failures, and only for the kind a person must clear.
#[tokio::test]
async fn a_human_required_condition_is_counted_as_a_challenge() {
    let (fetcher, _dir) = fetcher_with(Vec::new());
    fetcher
        .record_access_condition(
            "athletic.net",
            AccessBlockKind::HumanRequired,
            403,
            None,
            "challenge page",
        )
        .await;
    fetcher
        .record_access_condition(
            "athletic.net",
            AccessBlockKind::RateLimited,
            429,
            Some(30),
            "slow down",
        )
        .await;

    let stats = fetcher.stats().await;
    assert_eq!(
        stats.challenges, 1,
        "the challenge is counted, the rate limit is not"
    );
    assert_eq!(
        stats.rate_limited, 0,
        "a 429 is counted where the fetch saw it, not where the condition was recorded"
    );
}
