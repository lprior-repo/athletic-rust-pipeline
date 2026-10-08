use super::*;

const HTML: &str = "<details><summary>Published High School</summary><table><tr><td>Boys Cross Country</td><td>Head Coach</td><td>Published Coach</td><td></td></tr></table></details>";

fn outcome() -> FetchOutcome {
    FetchOutcome {
        url: URL.to_owned(),
        response_url: None,
        method: "GET".to_owned(),
        status: 200,
        content_digest: artifacts::sha(HTML.as_bytes()),
        bytes: HTML.len(),
        fetched_at: "2026-10-02T23:59:58Z".to_owned(),
        from_cache: false,
        content_type: Some("text/html".to_owned()),
        body: HTML.as_bytes().to_vec(),
    }
}

fn stats() -> Result<FetchStats> {
    Ok(FetchStats {
        requests: 2,
        bytes_downloaded: u64::try_from(HTML.len())?,
        ..FetchStats::default()
    })
}

fn clock(day: &str, time: &str, uptime: u64) -> Value {
    json!({"date":day,"realtime":format!("{day}T{time}Z"),
        "boot_id":"same-guest-boot","machine_id":"same-guest-machine",
        "monotonic_uptime":format!("{uptime}.00 0.00")})
}

fn record(phase: Phase) -> Result<Record> {
    let (day, start, end, uptime) = match phase {
        Phase::Before => ("2026-10-02", "23:59:57", "23:59:59", 100),
        Phase::After => ("2026-10-03", "00:00:01", "00:00:03", 104),
    };
    let mut capture = outcome();
    if phase == Phase::After {
        capture.fetched_at = "2026-10-03T00:00:02Z".to_owned();
    }
    Ok(Record {
        phase,
        identity: Identity {
            run: ENDPOINT.to_owned(),
            season: 2026,
            cohort: 2027,
            revision: 1,
            source: census_crawl::riil::SOURCE_ID.to_owned(),
            source_unit: format!("{ENDPOINT}:RI:{}:{URL}", census_crawl::riil::SOURCE_ID),
            manifest_sha256: "same-immutable-manifest".to_owned(),
        },
        capture,
        paths: CapturePaths {
            body: PathBuf::new(),
            metadata: PathBuf::new(),
            metadata_sha256: String::new(),
            archive_body: PathBuf::new(),
            archive_metadata: PathBuf::new(),
        },
        journal: JournalRef {
            store_root: PathBuf::new(),
            phase: JOURNAL.to_owned(),
            key: String::new(),
            readback: PathBuf::new(),
        },
        clock_before: clock(day, start, uptime),
        clock: clock(day, end, uptime.saturating_add(2)),
        stats: stats()?,
        schools: 1,
        xc_tf_appointments: 1,
    })
}

#[test]
fn rejects_unknown_phase_before_acquiring_resources() {
    assert!(Phase::parse("retry").is_err());
    assert!(Phase::parse("Before").is_err());
    assert_eq!(Phase::parse("after").ok(), Some(Phase::After));
}

#[test]
fn cached_and_conditional_responses_cannot_prove_a_new_physical_body() -> Result<()> {
    let mut capture = outcome();
    capture.from_cache = true;
    assert!(checks::capture(&capture, &stats()?).is_err());
    capture.from_cache = false;
    let conditional = FetchStats {
        conditional_304: 1,
        ..stats()?
    };
    assert!(checks::capture(&capture, &conditional).is_err());
    Ok(())
}

#[test]
fn corrupt_or_non_directory_bytes_do_not_count_as_acquisition_data() -> Result<()> {
    let mut capture = outcome();
    capture.body.push(b'x');
    assert!(checks::capture(&capture, &stats()?).is_err());
    let body = b"<html>Access denied</html>".to_vec();
    capture.bytes = body.len();
    capture.content_digest = artifacts::sha(&body);
    capture.body = body;
    let traffic = FetchStats {
        bytes_downloaded: u64::try_from(capture.bytes)?,
        ..stats()?
    };
    assert!(checks::capture(&capture, &traffic).is_err());
    Ok(())
}

#[test]
fn original_timestamp_must_fall_inside_measured_guest_interval() -> Result<()> {
    let before = clock("2026-10-02", "23:59:57.500", 100);
    let after = clock("2026-10-02", "23:59:59", 102);
    let mut capture = outcome();
    checks::acquisition_time(&capture, &before, &after)?;
    capture.fetched_at = "2026-10-01T23:59:58Z".to_owned();
    assert!(checks::acquisition_time(&capture, &before, &after).is_err());
    capture.fetched_at = "2026-10-03T00:00:01Z".to_owned();
    assert!(checks::acquisition_time(&capture, &before, &after).is_err());
    Ok(())
}

#[test]
fn crossing_requires_successive_dates_and_stable_domain_identity() -> Result<()> {
    let before = record(Phase::Before)?;
    let mut after = record(Phase::After)?;
    checks::crossing(&before, &after)?;
    after.capture.fetched_at = before.capture.fetched_at.clone();
    assert!(checks::crossing(&before, &after).is_err());
    after.capture.fetched_at = "2026-10-04T00:00:02Z".to_owned();
    assert!(checks::crossing(&before, &after).is_err());
    after.capture.fetched_at = "2026-10-03T00:00:02Z".to_owned();
    after.identity.cohort = 2028;
    assert!(checks::crossing(&before, &after).is_err());
    Ok(())
}

#[test]
fn reboot_is_not_a_natural_midnight_acquisition_witness() -> Result<()> {
    let before = record(Phase::Before)?;
    let mut after = record(Phase::After)?;
    after.clock_before["boot_id"] = json!("replacement-boot");
    assert!(checks::crossing(&before, &after).is_err());
    Ok(())
}

#[test]
fn acquisition_root_cannot_alias_an_endpoint_database() -> Result<()> {
    let acquisition = tempfile::tempdir()?;
    let other = tempfile::tempdir()?;
    std::os::unix::fs::symlink(other.path(), acquisition.path().join("fjall"))?;
    assert!(persistence::ensure_owned_directories(acquisition.path()).is_err());
    Ok(())
}

#[test]
fn fresh_directory_body_with_policy_traffic_is_a_physical_acquisition() -> Result<()> {
    let capture = outcome();
    let traffic = stats()?;
    assert_eq!(checks::capture(&capture, &traffic)?, (1, 1));
    let additional = FetchStats {
        requests: 3,
        ..traffic
    };
    assert!(checks::capture(&capture, &additional).is_err());
    Ok(())
}
