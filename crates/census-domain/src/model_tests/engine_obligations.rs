use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn digest(seed: u8) -> String {
    let mut out = String::with_capacity(64);
    for i in 0..64 {
        let v = (i as u8).wrapping_add(seed) % 16;
        let c = if v < 10 { b'0' + v } else { b'a' + (v - 10) };
        out.push(c as char);
    }
    out
}

fn claim(seed: u8) -> TestResult<PerformanceClaim> {
    PerformanceClaim::new(PerformanceRequest {
        athlete_id: "ath_perf_01".to_string(),
        meet_id: "meet_perf_02".to_string(),
        event: EventKind::Track1600m,
        date: "2026-05-02".to_string(),
        source_key: format!("src-{seed}"),
        capture_digest: digest(seed),
        corroboration: Corroboration::independent(),
    })
    .map_err(Into::into)
}

#[test]
fn performance_claims_keep_stable_ids_and_event_compatibility() -> TestResult {
    let first = claim(1)?;
    let repeat = PerformanceClaim::new(PerformanceRequest {
        athlete_id: "ath_perf_01".to_string(),
        meet_id: "meet_perf_02".to_string(),
        event: EventKind::Track1600m,
        date: "2026-05-02".to_string(),
        source_key: "src-1".to_string(),
        capture_digest: digest(9),
        corroboration: Corroboration::independent(),
    })?;
    check!(eq; first.id(), repeat.id());
    let other_event = PerformanceClaim::new(PerformanceRequest {
        athlete_id: "ath_perf_01".to_string(),
        meet_id: "meet_perf_02".to_string(),
        event: EventKind::Track3200m,
        date: "2026-05-02".to_string(),
        source_key: "src-1".to_string(),
        capture_digest: digest(1),
        corroboration: Corroboration::independent(),
    })?;
    check!(!first.same_event(&other_event));
    check!(first.same_event(&repeat));
    check!(PerformanceClaim::new(PerformanceRequest {
        athlete_id: "ath_perf_01".to_string(),
        meet_id: "meet_perf_02".to_string(),
        event: EventKind::Track1600m,
        date: "not-a-date".to_string(),
        source_key: "src-1".to_string(),
        capture_digest: digest(1),
        corroboration: Corroboration::independent(),
    })
    .is_err());
    let syndicated = Corroboration::syndicated("wire-family".to_string())?;
    check!(!syndicated.is_independent());
    check!(Corroboration::syndicated("".to_string()).is_err());
    let encoded = serde_json::to_string(&first)?;
    let decoded: PerformanceClaim = serde_json::from_str(&encoded)?;
    check!(eq; decoded, first);
    Ok(())
}

#[test]
fn coverage_obligations_move_once_from_pending_to_terminal() -> TestResult {
    let pending = CoverageObligation::pending(PendingObligation {
        id: "jurisdiction:wi:2026-outdoor:1".to_string(),
        scope: CoverageScope::Jurisdiction,
        subject: "wi".to_string(),
        season: "2026-outdoor".to_string(),
        revision: 1,
        detail: "schools remain to discover".to_string(),
    })?;
    check!(!pending.is_terminal());
    let done = pending.complete(digest(3))?;
    check!(done.is_terminal());
    check!(eq; done.evidence_digest(), Some(digest(3).as_str()));
    check!(done.complete(digest(4)).is_err());
    check!(done
        .block(AccessBlockKind::RateLimited, "limited".to_string())
        .is_err());
    let blocked = pending.block(AccessBlockKind::RateLimited, "429 cooldown".to_string())?;
    check!(blocked.is_terminal());
    let gap = pending.retain_gap("source unreachable, work remains".to_string())?;
    check!(gap.is_terminal());
    let encoded = serde_json::to_string(&done)?;
    let decoded: CoverageObligation = serde_json::from_str(&encoded)?;
    check!(eq; decoded, done);
    Ok(())
}

#[test]
fn coverage_blocks_cover_every_access_kind_without_becoming_complete() -> TestResult {
    for kind in [
        AccessBlockKind::Forbidden,
        AccessBlockKind::RateLimited,
        AccessBlockKind::Timeout,
        AccessBlockKind::Unavailable,
        AccessBlockKind::BrowserUnavailable,
        AccessBlockKind::HumanRequired,
    ] {
        let pending = CoverageObligation::pending(PendingObligation {
            id: "source-sweep:milesplit:wi:2026-outdoor:1".to_string(),
            scope: CoverageScope::Source,
            subject: "milesplit:wi".to_string(),
            season: "2026-outdoor".to_string(),
            revision: 1,
            detail: "sweep open".to_string(),
        })?;
        let blocked = pending.block(kind, "stop for one source".to_string())?;
        check!(blocked.is_terminal());
        check!(eq; blocked.evidence_digest(), None);
    }
    Ok(())
}
