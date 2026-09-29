use super::*;

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

#[test]
fn the_efficiency_ratio_is_absent_without_physical_requests() {
    let stats = FetchStats::default();
    assert_eq!(stats.physical_requests(), 0);
    assert_eq!(stats.useful_records_per_physical_request(4_000), None);
}

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

#[test]
fn a_percentile_of_no_samples_is_absent() {
    let stats = FetchStats::default();
    assert_eq!(stats.latency_samples(), 0);
    assert_eq!(stats.latency_ms_avg(), None);
    assert_eq!(stats.latency_percentile_ms(99), None);
}

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
