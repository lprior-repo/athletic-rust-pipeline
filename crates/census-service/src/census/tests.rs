//! Tests for the §45 transport account: the reading a run's client counters become in a report.

use super::TransportReport;
use census_crawl::net::{FetchStats, HostTraffic};

/// §45: the account reads the client's counters once, and its per-source rows account for the
/// traffic rather than approximating it.
#[test]
fn the_transport_report_reads_the_counters_once() {
    let mut stats = FetchStats {
        requests: 5,
        cache_hits: 2,
        bytes_downloaded: 900,
        ..FetchStats::default()
    };
    stats.per_host.insert(
        "b.example".to_string(),
        HostTraffic {
            requests: 1,
            cache_hits: 0,
            bytes: 100,
        },
    );
    stats.per_host.insert(
        "a.example".to_string(),
        HostTraffic {
            requests: 4,
            cache_hits: 2,
            bytes: 800,
        },
    );

    let report = TransportReport::from_stats(&stats, 6);
    assert_eq!(report.requests, 5);
    assert_eq!(report.cache_hits, 2);
    assert_eq!(report.physical_requests, 3);
    assert_eq!(report.bytes, 900);
    assert_eq!(report.verified_records, 6);
    // Busiest first, so two runs with the same traffic render the same rows.
    let hosts: Vec<&str> = report.sources.iter().map(|row| row.host.as_str()).collect();
    assert_eq!(hosts, vec!["a.example", "b.example"]);
    let per_source: u64 = report
        .sources
        .iter()
        .map(|row| row.physical_requests)
        .fold(0, u64::saturating_add);
    assert_eq!(
        per_source, report.physical_requests,
        "the rows account for the run's physical traffic"
    );
    // Nothing was measured, so there is no percentile to print — not a zero.
    assert_eq!(report.p95_latency_ms, None);
    assert!(matches!(
        report.verified_records_per_physical_request,
        Some(ratio) if (ratio - 2.0).abs() < 1e-9
    ));
}
