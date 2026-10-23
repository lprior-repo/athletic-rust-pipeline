use super::*;
type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

fn target() -> Result<Target, census_domain::model::CaptureDomainError> {
    let packet = AcquisitionDigest::parse(
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    )?;
    Ok(Target {
        packet: packet.clone(),
        athlete_id: "9401".into(),
        next: Cursor {
            packet,
            member: 1,
            finished: false,
        },
    })
}

fn denied(refusal: access::Refusal) -> CaptureOutcome {
    CaptureOutcome::SourceFailure {
        message: "controlled source refusal".into(),
        refusal,
        retryable: false,
    }
}

#[test]
fn profile_forbidden_streak_survives_serialization_and_stops_on_fifth_page() -> TestResult {
    let mut progress = Progress::default();
    for _ in 0..4 {
        progress.record(target()?, denied(access::Refusal::Page))?;
    }
    check!(!progress.stopped);
    let mut resumed: Progress = serde_json::from_slice(&serde_json::to_vec(&progress)?)?;
    resumed.record(target()?, denied(access::Refusal::Page))?;
    check!(resumed.stopped);
    check!(eq; resumed.failed, 5);
    check!(eq; resumed.cursor, Some(target()?.next));
    Ok(())
}

#[test]
fn profile_host_refusal_stops_immediately_and_persists_the_stop_reason() -> TestResult {
    let mut progress = Progress::default();
    progress.record(target()?, denied(access::Refusal::Host))?;
    let resumed: Progress = serde_json::from_slice(&serde_json::to_vec(&progress)?)?;
    check!(resumed.stopped);
    check!(eq; resumed.failed, 1);
    check!(resumed.last_failure.is_some());
    check!(!resumed.exhausted);
    Ok(())
}

#[test]
fn successful_profile_breaks_forbidden_streak_without_erasing_failure_gaps() -> TestResult {
    let mut progress = Progress::default();
    for _ in 0..4 {
        progress.record(target()?, denied(access::Refusal::Page))?;
    }
    progress.record(target()?, CaptureOutcome::Captured)?;
    for _ in 0..4 {
        progress.record(target()?, denied(access::Refusal::Page))?;
    }
    check!(!progress.stopped);
    check!(eq; progress.failed, 8);
    check!(eq; progress.captured, 1);
    progress.record(target()?, denied(access::Refusal::Page))?;
    check!(progress.stopped);
    Ok(())
}

#[test]
fn profile_counter_exhaustion_cannot_advance_cursor() -> TestResult {
    let mut progress = Progress {
        captured: u64::MAX,
        ..Progress::default()
    };
    check!(progress
        .record(target()?, CaptureOutcome::Captured)
        .is_err());
    check!(progress.cursor.is_none());
    check!(eq; progress.captured, u64::MAX);
    let mut progress = Progress {
        source_gaps: u64::MAX,
        ..Progress::default()
    };
    check!(progress.advance(target()?.next, 1).is_err());
    check!(progress.cursor.is_none());
    Ok(())
}

#[test]
fn incomplete_roster_inventory_waits_then_rescans_new_progress_without_resetting_work_counts(
) -> TestResult {
    let mut progress = Progress::default();
    let first = target()?.packet;
    progress.bind_rosters(first.clone())?;
    progress.record(target()?, CaptureOutcome::Captured)?;
    progress.finish_scan(false);
    let mut progress: Progress = serde_json::from_slice(&serde_json::to_vec(&progress)?)?;
    check!(progress.awaiting_rosters);
    check!(!progress.exhausted);
    progress.bind_rosters(first)?;
    check!(progress.awaiting_rosters);
    check!(progress.cursor.is_some());
    let changed = AcquisitionDigest::parse(
        "1123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    )?;
    progress.bind_rosters(changed)?;
    check!(!progress.awaiting_rosters);
    check!(progress.cursor.is_none());
    check!(eq; progress.scan_rounds, 2);
    check!(eq; progress.captured, 1);
    progress.finish_scan(true);
    check!(progress.exhausted);
    Ok(())
}

#[test]
fn changed_roster_progress_cannot_clear_an_existing_source_stop() -> TestResult {
    let mut progress = Progress::default();
    progress.bind_rosters(target()?.packet)?;
    progress.record(target()?, denied(access::Refusal::Host))?;
    let changed = AcquisitionDigest::parse(
        "1123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    )?;
    progress.bind_rosters(changed)?;
    check!(progress.stopped);
    check!(eq; progress.failed, 1);
    Ok(())
}
