//! The streaming merge holds one id; the collecting scan holds the table.
//!
//! Two passes over their own freshly opened store hold the same rows: `for_each_merged` merges one id
//! at a time and drops the previous row before reading the next, while `scan` collects every merged
//! row into a `Vec`. The property this test pins is the difference between them, measured on the
//! process's own resident set rather than through an allocator hook: `#![forbid(unsafe_code)]` is a
//! repository rule, so `/proc/self/status` is read instead. Page granularity is coarse, which is why
//! the table is built large enough to be unmistakable — the two passes' block-cache growth cancels
//! because each pass opens its own store, and the rows themselves do not.
//!
//! Linux-only: the reading is `/proc`, and a platform without it skips rather than pretends.

use std::path::Path;

use census_domain::jurisdiction::UsJurisdiction;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, CentiSeconds, EventKind, Gender, GradYear, Mark, SchoolYear, Sport,
    TimingMethod,
};
use census_store::{Store, Table};

/// Athletes, hence distinct merged performances: enough that the collected table is megabytes.
const ATHLETES: usize = 400;

/// Observations per athlete: the merge reads them, the store keeps one row per athlete.
const VERSIONS: usize = 8;

/// One performance per athlete and version, keyed so each athlete's versions share one id.
fn performance(index: usize, version: usize) -> CanonicalPerformance {
    let school = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
    );
    let athlete =
        CanonicalAthlete::mint(&school, "Julian Aguilera", GradYear::CO2027, Gender::Boys);
    let meet = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-01",
        "Abbotsford Invite",
        None,
    );
    let team = CanonicalTeam::mint(
        &school,
        Sport::OutdoorTrack,
        Gender::Boys,
        SchoolYear::new(2026).expect("2026 is a season"),
    );
    CanonicalPerformance {
        id: CanonicalPerformance::mint(
            &athlete,
            &meet,
            &EventKind::Track100m,
            "2026-05-01",
            &format!("athlete-{index}"),
        ),
        athlete,
        team,
        event: CanonicalEvent::new(&meet, EventKind::Track100m, Gender::Boys, None, None).id,
        meet,
        date: "2026-05-01".to_string(),
        mark: Mark::TimeSeconds(CentiSeconds::from_seconds_f64(
            10.94 + (version as f64) / 100.0,
        )),
        wind_mps: None,
        place: Some((version + 1) as u16),
        heat: None,
        round: None,
        timing: Some(TimingMethod::Fat),
        observed_grade: None,
        evidence: Vec::new(),
        source_key: format!("athlete-{index}"),
        source_athlete: None,
        retained_conflicts: Vec::new(),
    }
}

/// Every observation the two passes read, in one batch per athlete.
fn corpus() -> Vec<Vec<CanonicalPerformance>> {
    (0..ATHLETES)
        .map(|index| {
            (0..VERSIONS)
                .map(|version| performance(index, version))
                .collect()
        })
        .collect()
}

/// A store holding the corpus, opened fresh so no other test's pages are resident.
fn seeded(root: &Path) -> Store {
    let store = Store::open(root).expect("the store opens");
    for batch in corpus() {
        store
            .append_many(Table::Performances, &batch)
            .expect("the batch appends");
    }
    store
}

/// The process's resident set in bytes, or `None` off Linux.
fn resident_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

#[test]
fn streaming_merge_does_not_materialize_the_table() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let streamed_store = seeded(&dir.path().join("streamed"));

    // Pass 1: the streaming merge, holding nothing but the id in hand.
    let before = resident_bytes();
    let mut visited = Vec::new();
    streamed_store
        .for_each_merged(Table::Performances, |row: CanonicalPerformance| {
            visited.push(row.id.as_str().to_string());
            Ok(())
        })
        .expect("the streaming pass reads the table");
    let after_stream = resident_bytes();

    // Pass 2: the collecting scan, holding every merged row.
    let collected_store = seeded(&dir.path().join("collected"));
    let before_scan = resident_bytes();
    let collected: Vec<CanonicalPerformance> = collected_store
        .scan(Table::Performances)
        .expect("the collecting pass reads the table");
    let after_scan = resident_bytes();

    // The two passes read the same table: same ids, same order, one row per athlete.
    let collected_ids: Vec<String> = collected
        .iter()
        .map(|row| row.id.as_str().to_string())
        .collect();
    assert_eq!(
        visited, collected_ids,
        "the streaming pass yields exactly the rows the scan collects"
    );
    assert_eq!(visited.len(), ATHLETES, "one merged row per athlete");

    let serialized: usize = collected
        .iter()
        .map(|row| serde_json::to_vec(row).expect("a row serializes").len())
        .sum();

    let (Some(before), Some(after_stream), Some(before_scan), Some(after_scan)) =
        (before, after_stream, before_scan, after_scan)
    else {
        eprintln!("skipped: /proc/self/status is not readable on this platform");
        return;
    };
    let streamed = after_stream.saturating_sub(before);
    let scanned = after_scan.saturating_sub(before_scan);
    eprintln!(
        "resident growth: streaming {streamed} bytes, collecting {scanned} bytes, \
         rows serialize to {serialized} bytes"
    );
    assert!(
        scanned >= streamed + (serialized as u64) / 2,
        "the collecting pass pays for the table and the streaming pass does not: \
         streaming {streamed}, collecting {scanned}, rows {serialized}"
    );
}
