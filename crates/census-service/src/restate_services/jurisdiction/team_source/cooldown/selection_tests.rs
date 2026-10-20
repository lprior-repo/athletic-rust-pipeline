use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const NOW: &str = "2026-10-14T11:30:00Z";

fn row(
    host: &str,
    kind: census_domain::model::AccessBlockKind,
    until: Option<&str>,
) -> SourceAccessCondition {
    SourceAccessCondition::new("retained-provider", host, kind, 403, NOW, "retained detail")
        .with_cooldown_until(until.map(str::to_owned))
}

#[test]
fn registered_host_not_evidence_source_label_controls_deferral() -> TestResult {
    use census_domain::model::AccessBlockKind::Forbidden;
    let foreign = row("api.ihsa.org.evil.test", Forbidden, None);
    let unrelated = row("other.test", Forbidden, None);
    let matching = row("API.IHSA.ORG", Forbidden, Some("2026-10-14T12:00:00Z"));
    let rows = [foreign, unrelated, matching];
    check!(eq; active_block(&rows, NOW, "api.ihsa.org"), rows.last());
    check!(matches!(
        gate_decision(&rows, NOW, "api.ihsa.org", "ihsa"),
        GateDecision::Sleep {
            milliseconds: 1_800_000,
            ..
        }
    ));
    check!(matches!(
        gate_decision(&rows, NOW, "arbitersports.com", "arbiter_orgs"),
        GateDecision::Proceed
    ));
    Ok(())
}

#[test]
fn latest_deadline_then_smallest_id_is_independent_of_scan_order() -> TestResult {
    use census_domain::model::AccessBlockKind::{Forbidden, RateLimited, Timeout};
    let early = row("api.ihsa.org", Timeout, Some("2026-10-14T11:31:00Z"));
    let forbidden = row("api.ihsa.org", Forbidden, Some("2026-10-14T12:00:00Z"));
    let limited = row("api.ihsa.org", RateLimited, Some("2026-10-14T12:00:00Z"));
    for rows in [
        [early.clone(), forbidden.clone(), limited.clone()],
        [limited.clone(), early.clone(), forbidden.clone()],
        [forbidden.clone(), limited.clone(), early.clone()],
        [limited.clone(), forbidden.clone(), early.clone()],
        [early.clone(), limited.clone(), forbidden.clone()],
        [forbidden.clone(), early.clone(), limited.clone()],
    ] {
        check!(eq; active_block(&rows, NOW, "api.ihsa.org"), Some(&forbidden));
    }
    Ok(())
}

#[test]
fn indefinite_and_malformed_timing_cannot_be_hidden_by_finite_deadlines() -> TestResult {
    use census_domain::model::AccessBlockKind::{Forbidden, HumanRequired, RateLimited};
    let finite = row("api.ihsa.org", Forbidden, Some("2026-10-14T12:00:00Z"));
    for blocked in [
        row("api.ihsa.org", HumanRequired, None),
        row("api.ihsa.org", RateLimited, Some("malformed")),
    ] {
        for rows in [
            [finite.clone(), blocked.clone()],
            [blocked.clone(), finite.clone()],
        ] {
            check!(eq; active_block(&rows, NOW, "api.ihsa.org"), Some(&blocked));
            check!(matches!(
                gate_decision(&rows, NOW, "api.ihsa.org", "ihsa"),
                GateDecision::Blocked { .. }
            ));
        }
    }
    let rows = [finite];
    check!(matches!(
        gate_decision(&rows, "invalid", "api.ihsa.org", "ihsa"),
        GateDecision::Blocked { .. }
    ));
    Ok(())
}

#[test]
fn expiry_boundary_and_positive_submillisecond_wait_are_distinct() -> TestResult {
    use census_domain::model::AccessBlockKind::Forbidden;
    let rows = [row("api.ihsa.org", Forbidden, Some("2026-10-14T11:30:00Z"))];
    check!(matches!(
        gate_decision(&rows, NOW, "api.ihsa.org", "ihsa"),
        GateDecision::Proceed
    ));
    let rows = [row(
        "api.ihsa.org",
        Forbidden,
        Some("2026-10-14T11:30:00.000001Z"),
    )];
    check!(matches!(
        gate_decision(&rows, NOW, "api.ihsa.org", "ihsa"),
        GateDecision::Sleep {
            milliseconds: 1,
            ..
        }
    ));
    check!(matches!(
        gate_decision(&[], NOW, "api.ihsa.org", "ihsa"),
        GateDecision::Proceed
    ));
    Ok(())
}

#[test]
fn unvalidated_settled_payload_cannot_bypass_the_gate() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let mut request = super::super::tests::request()?;
    request.source = "ihsa".into();
    let operation = super::super::key(&request);
    let mut batch = store.write_batch();
    batch.journal_done(
        ledger::PHASE,
        &format!("{operation}/settled"),
        &serde_json::json!({"invented": true}),
    )?;
    batch.commit()?;
    let result = read_decision(
        &store,
        &census_store::clock::SystemClock,
        &operation,
        &request,
    );
    check!(matches!(result, Err(JobError::Terminal { .. })));
    Ok(())
}
