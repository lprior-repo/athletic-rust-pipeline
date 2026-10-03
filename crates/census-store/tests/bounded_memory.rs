#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use std::path::Path;

use census_domain::jurisdiction::UsJurisdiction;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, CentiSeconds, EventKind, Gender, GradYear, Mark, SchoolYear, SourceIdentity,
    SourceNamespace, Sport, TimingMethod,
};
use census_store::{Store, Table};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const ATHLETES: usize = 400;
const VERSIONS: usize = 8;

fn performance(index: usize, version: usize) -> TestResult<CanonicalPerformance> {
    let school = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
    );
    let identity = SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        format!("athlete-{index}"),
    );
    let athlete = CanonicalAthlete::mint(
        &school,
        "Julian Aguilera",
        GradYear::CO2027,
        Gender::Boys,
        &identity,
    );
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
        SchoolYear::new(2026).ok_or("2026 is a season")?,
    );
    Ok(CanonicalPerformance {
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
        mark: Mark::TimeSeconds(
            CentiSeconds::try_from_seconds_f64(10.94 + (version as f64) / 100.0)
                .ok_or("ten seconds is in range")?,
        ),
        wind_mps: None,
        place: Some((version + 1) as u16),
        heat: None,
        round: None,
        timing: Some(TimingMethod::Fat),
        observed_grade: None,
        evidence: Vec::new(),
        source_key: format!("athlete-{index}"),
        source_athlete: Some(identity),
        retained_conflicts: Vec::new(),
    })
}

fn corpus() -> TestResult<Vec<Vec<CanonicalPerformance>>> {
    (0..ATHLETES)
        .map(|index| {
            (0..VERSIONS)
                .map(|version| performance(index, version))
                .collect()
        })
        .collect()
}

fn seeded(root: &Path) -> TestResult<Store> {
    let store = Store::open(root)?;
    for batch in corpus()? {
        store.append_many(Table::Performances, &batch)?;
    }
    Ok(store)
}

fn resident_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

#[test]
fn streaming_merge_does_not_materialize_the_table() -> TestResult {
    let dir = tempfile::tempdir()?;
    let streamed_store = seeded(&dir.path().join("streamed"))?;
    let before = resident_bytes();
    let mut visited = Vec::new();
    streamed_store.for_each_merged(Table::Performances, |row: CanonicalPerformance| {
        visited.push(row.id.as_str().to_string());
        Ok(())
    })?;
    let after_stream = resident_bytes();
    let collected_store = seeded(&dir.path().join("collected"))?;
    let before_scan = resident_bytes();
    let collected: Vec<CanonicalPerformance> = collected_store.scan(Table::Performances)?;
    let after_scan = resident_bytes();
    let collected_ids: Vec<String> = collected
        .iter()
        .map(|row| row.id.as_str().to_string())
        .collect();
    check!(eq; visited, collected_ids, "the streaming pass yields exactly the rows the scan collects");
    check!(eq; visited.len(), ATHLETES, "one merged row per athlete");
    let serialized: usize =
        collected
            .iter()
            .try_fold(0, |total, row| -> Result<usize, serde_json::Error> {
                Ok(total + serde_json::to_vec(row)?.len())
            })?;
    let (Some(before), Some(after_stream), Some(before_scan), Some(after_scan)) =
        (before, after_stream, before_scan, after_scan)
    else {
        eprintln!("skipped: /proc/self/status is not readable on this platform");
        return Ok(());
    };
    let streamed = after_stream.saturating_sub(before);
    let scanned = after_scan.saturating_sub(before_scan);
    eprintln!("resident growth: streaming {streamed} bytes, collecting {scanned} bytes, rows serialize to {serialized} bytes");
    check!(scanned >= streamed + (serialized as u64) / 2, "the collecting pass pays for the table and the streaming pass does not: streaming {streamed}, collecting {scanned}, rows {serialized}");
    Ok(())
}
