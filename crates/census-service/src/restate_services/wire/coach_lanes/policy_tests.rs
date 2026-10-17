use super::policy::Acquisition;
use super::{bind, co, il, sd, ut};
use crate::restate_services::wire::SourcePlan;
use census_domain::model::{CoachLaneError, CoachLaneMailbox, CoachLaneStatus, SchoolYear};
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn retained_policy_preserves_capture_zero_and_inclusive_freshness_boundaries() -> TestResult {
    let season = SchoolYear::new(2026).ok_or("season")?;
    for (row, captured, boundary, stale, schools, contacts, mailbox) in [
        (&co::POLICY, "2026-09-27", "2027-11-01", "2027-11-02", 378, 50, CoachLaneMailbox::Unpublished),
        (&il::POLICY, "2026-09-22", "2027-10-27", "2027-10-28", 3, 3, CoachLaneMailbox::Published),
        (&ut::POLICY, "2026-10-04", "2027-11-08", "2027-11-09", 162, 12, CoachLaneMailbox::Published),
        (&sd::POLICY, "2026-10-07", "2027-11-11", "2027-11-12", 3, 6, CoachLaneMailbox::Unpublished),
    ] {
        for (date, freshness) in [(captured, 0), (boundary, 400)] {
            let lane = row.evaluate(season, 1, date, Acquisition::Available)?;
            check!(eq; lane.status(), CoachLaneStatus::Armed);
            check!(eq; lane.resolution().freshness_days, freshness);
            check!(eq; lane.resolution().mailbox, mailbox);
            let evidence = lane.evidence().ok_or("retained evidence")?;
            check!(eq; evidence.schools(), schools);
            check!(eq; evidence.contacts(), contacts);
            check!(eq; evidence.observed_on(), captured);
        }
        let expired = row.evaluate(season, 1, stale, Acquisition::Available)?;
        check!(eq; expired.status(), CoachLaneStatus::Blocked);
        check!(expired.evidence().is_none());
        let refusal = expired.refusal().ok_or("stale refusal")?;
        check!(eq; refusal.source().id.as_str(), row.arm);
        check!(refusal.reason().contains("is 401 days old at lane creation, beyond the 400-day bound"));
        check!(eq; row.evaluate(season, 1, "2026-09-01", Acquisition::Available),
            Err(CoachLaneError::OutOfRange { field: "resolution.freshness_days" }));
    }
    Ok(())
}

#[test]
fn south_dakota_hold_precedes_freshness_but_not_pending_validation() -> TestResult {
    let season = SchoolYear::new(2026).ok_or("season")?;
    for date in ["2026-09-01", "2026-10-07", "2027-11-12"] {
        let lane = sd::POLICY.evaluate(season, 1, date, Acquisition::Held("held by policy"))?;
        check!(eq; lane.status(), CoachLaneStatus::Blocked);
        check!(eq; lane.refusal().map(|refusal| refusal.reason()), Some("held by policy"));
        check!(eq; lane.resolution().freshness_days, 0);
    }
    check!(eq; sd::POLICY.evaluate(season, 0, "invalid", Acquisition::Held("held by policy")),
        Err(CoachLaneError::OutOfRange { field: "revision" }));
    Ok(())
}

#[test]
fn all_failed_bindings_preserve_plan_and_historical_validation_order() -> TestResult {
    let season = SchoolYear::new(2026).ok_or("season")?;
    for jurisdiction in UsJurisdiction::CENSUS_SCOPE {
        let mut plan = SourcePlan::default();
        let before = serde_json::to_vec(&plan)?;
        let field = match jurisdiction {
            UsJurisdiction::Arizona | UsJurisdiction::California | UsJurisdiction::Colorado
            | UsJurisdiction::Connecticut | UsJurisdiction::Florida | UsJurisdiction::Illinois
            | UsJurisdiction::Indiana | UsJurisdiction::Iowa | UsJurisdiction::Louisiana
            | UsJurisdiction::Massachusetts | UsJurisdiction::Michigan | UsJurisdiction::Missouri
            | UsJurisdiction::Nevada | UsJurisdiction::NewJersey | UsJurisdiction::NewYork
            | UsJurisdiction::Oklahoma | UsJurisdiction::Oregon | UsJurisdiction::SouthDakota
            | UsJurisdiction::Texas | UsJurisdiction::Utah | UsJurisdiction::Vermont
            | UsJurisdiction::Virginia | UsJurisdiction::Washington => "revision",
            _ => "refusal.observed_on",
        };
        check!(eq; bind(&mut plan, jurisdiction, season, 0, "invalid"),
            Err(CoachLaneError::OutOfRange { field }));
        check!(eq; serde_json::to_vec(&plan)?, before);
    }
    for jurisdiction in [UsJurisdiction::Alaska, UsJurisdiction::Hawaii] {
        let mut plan = SourcePlan::default();
        check!(eq; bind(&mut plan, jurisdiction, season, 0, "invalid")?, None);
        check!(eq; serde_json::to_vec(&plan)?, serde_json::to_vec(&SourcePlan::default())?);
    }
    Ok(())
}
