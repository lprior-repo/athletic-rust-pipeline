use census_domain::model::{
    CoachLaneError, CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution, CoachLaneStage,
    PendingCoachLane, SchoolYear, SourceRef,
};
use census_domain::UsJurisdiction;

pub(super) const CA_ARM: &str = "home_campus";
pub(super) const CA_REFUSAL_SLUG: &str = "ca_missing_coach_sources";
pub(super) const CA_TRUE_MISSING: &str = "CIF Oakland Section (`cifoakland.org`); PrepCalTrack SCVAL (`lynbrooksports.prepcaltrack.com`); USATF Pacific (`pausatf.org`); California Coaches Association; CIF-SS per-school details endpoint (405s today); CIF-SS directory sections 2-13 (405s today).";
#[cfg(test)]
pub(super) const CA_CAPTURE_PATH: &str = "var/home-campus-CA-20261004/pass2.log";
#[cfg(test)]
pub(super) const CA_CAPTURE_DIGEST: &str =
    "b1658ef1f62f9373fca3b4a1c4bd24b6c87a1e46818bea3ead435374c9da5cd4";

const CA_ARM_BASIS: &str = "home_campus is wired and remains sweepable, but var/home-campus-CA-20261004/pass2.log sha256 b1658ef1f62f9373fca3b4a1c4bd24b6c87a1e46818bea3ead435374c9da5cd4 records historical totals only (1717 schools, 6758 email rows); it supplies neither this run's executed obligation nor page-bound current role/tenure/mailbox proof, so those counts cannot arm or close a fresh coach lane";

pub(super) fn lane(
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    let pending = CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction: UsJurisdiction::California,
        season,
        revision,
        arm: CA_ARM.to_string(),
        arm_basis: CA_ARM_BASIS.to_string(),
        observed_on: observed_on.to_string(),
    })?;
    pending.record(
        CoachLaneStage::Blocked,
        Some(CoachLaneRefusal::new(
            SourceRef::id(CA_REFUSAL_SLUG),
            CA_TRUE_MISSING,
            observed_on,
        )?),
        CoachLaneResolution::unresolved(observed_on),
    )
}

#[cfg(test)]
mod tests {
    use super::super::{bind, capture_digest};
    use super::*;
    use crate::restate_services::jurisdiction::coach_lane_owed;
    use crate::restate_services::plan::{plan as planned, BrowserLaneState};
    use crate::restate_services::teams_arms;
    use crate::restate_services::wire::{JurisdictionState, SourcePlan};
    use census_domain::model::{CoachLaneMailbox, CoachLaneStatus, CoachLaneTenure};
    use sha2::{Digest, Sha256};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const OBSERVED_ON: &str = "2026-10-11";
    const CAPTURE_BYTES: usize = 665;
    const WORK_ORDER: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/sources/coach-coverage-bundle-20261004/state-work-orders/group-a.md"
    ));

    fn season() -> Result<SchoolYear, Box<dyn std::error::Error>> {
        Ok(SchoolYear::new(2026).ok_or("school year 2026")?)
    }

    fn bound() -> Result<(SourcePlan, CoachLaneObligation), Box<dyn std::error::Error>> {
        let mut plan = SourcePlan::of(
            &planned(UsJurisdiction::California, BrowserLaneState::Absent),
            "fixture".to_string(),
        );
        let obligation = bind(
            &mut plan,
            UsJurisdiction::California,
            season()?,
            1,
            OBSERVED_ON,
        )?
        .ok_or("the California lane is registered")?;
        Ok((plan, obligation))
    }

    #[test]
    fn coach_lane_ca_historical_counts_cannot_close_a_fresh_run() -> TestResult {
        let recorded = lane(season()?, 1, "2026-10-12")?;
        check!(eq; recorded.status(), CoachLaneStatus::Blocked);
        check!(!recorded.is_terminal());
        check!(recorded.evidence().is_none());
        check!(recorded.acceptance().is_none());
        check!(eq; recorded.refusal().map(CoachLaneRefusal::reason), Some(CA_TRUE_MISSING));
        Ok(())
    }

    #[test]
    fn coach_lane_ca_records_historical_evidence_as_a_gap_without_closing() -> TestResult {
        let (plan, armed) = bound()?;
        check!(eq; armed.id(), "coach_lane:CA:2026:1");
        check!(eq; armed.status(), CoachLaneStatus::Blocked);
        check!(!armed.is_terminal());
        check!(eq; armed.arm_name(), CA_ARM);
        check!(eq; armed.refusal().map(CoachLaneRefusal::reason), Some(CA_TRUE_MISSING));
        check!(armed.acceptance().is_none());
        check!(armed.evidence().is_none());
        check!(eq; armed.resolution().tenure, CoachLaneTenure::Unknown);
        check!(eq; armed.resolution().mailbox, CoachLaneMailbox::Unpublished);
        check!(armed.arm_basis().contains(CA_CAPTURE_PATH));
        check!(armed.arm_basis().contains(CA_CAPTURE_DIGEST));
        check!(armed.arm_basis().contains("this run's executed obligation"));
        let mut state = JurisdictionState {
            plan: Some(plan),
            ..JurisdictionState::default()
        };
        check!(coach_lane_owed(&state));
        state.coach_lane = Some(armed);
        check!(coach_lane_owed(&state));
        Ok(())
    }

    #[test]
    fn coach_lane_ca_evidence_digest_is_the_sha256_of_the_capture() -> TestResult {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bytes = std::fs::read(root.join(CA_CAPTURE_PATH))?;
        check!(eq; bytes.len(), CAPTURE_BYTES);
        check!(eq; format!("{:x}", Sha256::digest(&bytes)).as_str(), CA_CAPTURE_DIGEST);
        check!(eq; capture_digest(CA_CAPTURE_PATH)?, CA_CAPTURE_DIGEST);
        Ok(())
    }

    #[test]
    fn coach_lane_ca_arm_is_wired_into_the_plan_and_the_engine_arm_table() -> TestResult {
        let (plan, armed) = bound()?;
        check!(plan.sweepable.iter().any(|slug| slug == CA_ARM));
        check!(teams_arms::arm_for(CA_ARM).is_some());
        check!(eq; armed.status(), CoachLaneStatus::Blocked);
        Ok(())
    }

    #[test]
    fn coach_lane_ca_stale_capture_records_a_blocked_refusal_without_closing() -> TestResult {
        let fresh = lane(season()?, 1, "2027-11-08")?;
        check!(eq; fresh.status(), CoachLaneStatus::Blocked);
        let stale = lane(season()?, 1, "2027-11-09")?;
        check!(eq; stale.status(), CoachLaneStatus::Blocked);
        check!(!stale.is_terminal());
        check!(stale.evidence().is_none());
        check!(stale.acceptance().is_none());
        let refusal = stale.refusal().ok_or("a stale capture records a refusal")?;
        check!(eq; refusal.reason(), CA_TRUE_MISSING);
        check!(stale.arm_basis().contains(CA_CAPTURE_PATH));
        let state = JurisdictionState {
            coach_lane: Some(stale),
            ..JurisdictionState::default()
        };
        check!(coach_lane_owed(&state));
        Ok(())
    }

    #[test]
    fn coach_lane_ca_true_missing_reason_is_the_work_order_line() -> TestResult {
        check!(WORK_ORDER.contains(CA_TRUE_MISSING));
        check!(WORK_ORDER.contains("CIF Oakland Section"));
        Ok(())
    }

    #[test]
    fn coach_lane_ca_refusal_is_recorded_in_the_durable_plan_path() -> TestResult {
        let (plan, _) = bound()?;
        check!(plan
            .refused
            .iter()
            .any(|entry| entry.slug == CA_REFUSAL_SLUG && entry.reason == CA_TRUE_MISSING));
        let refusals = plan.coach_lane_refusals(OBSERVED_ON)?;
        check!(refusals
            .iter()
            .any(|refusal| refusal.source().id == CA_REFUSAL_SLUG
                && refusal.reason() == CA_TRUE_MISSING));
        let unarmed = JurisdictionState {
            plan: Some(plan),
            ..JurisdictionState::default()
        };
        check!(coach_lane_owed(&unarmed));
        Ok(())
    }

    #[test]
    fn coach_lane_ca_refusal_binds_even_without_a_sweepable_arm() -> TestResult {
        let mut plan = SourcePlan {
            sweepable: Vec::new(),
            refused: Vec::new(),
            fingerprint: "fixture".to_string(),
        };
        let recorded = bind(
            &mut plan,
            UsJurisdiction::California,
            season()?,
            1,
            OBSERVED_ON,
        )?
        .ok_or("California refusal")?;
        check!(eq; recorded.status(), CoachLaneStatus::Blocked);
        check!(!recorded.is_terminal());
        Ok(())
    }

    #[test]
    fn coach_lane_bind_leaves_excluded_jurisdictions_unwired() -> TestResult {
        for jurisdiction in [UsJurisdiction::Alaska, UsJurisdiction::Hawaii] {
            let mut plan = SourcePlan::of(
                &planned(jurisdiction, BrowserLaneState::Absent),
                "fixture".to_string(),
            );
            let before = serde_json::to_vec(&plan)?;
            check!(bind(&mut plan, jurisdiction, season()?, 1, OBSERVED_ON)?.is_none());
            check!(eq; serde_json::to_vec(&plan)?, before);
        }
        Ok(())
    }

    #[test]
    fn coach_lane_ca_refusal_survives_store_reopen_and_rebind() -> TestResult {
        use census_store::{Store, Table};

        let root = tempfile::tempdir()?;
        let (plan, recorded) = bound()?;
        let state = JurisdictionState {
            identity: recorded.id().to_string(),
            plan: Some(plan),
            coach_lane: Some(recorded.clone()),
            ..JurisdictionState::default()
        };
        {
            let store = Store::open(root.path())?;
            store.replace(Table::CoachLanes, &recorded)?;
            store.journal_done("ca-coach-qualification", &state.identity, &state)?;
            store.flush()?;
        }
        let store = Store::open(root.path())?;
        check!(eq; store.scan::<CoachLaneObligation>(Table::CoachLanes)?, vec![recorded]);
        let payload = store
            .journal_payload("ca-coach-qualification", &state.identity)?
            .ok_or("California refusal lost on reopen")?;
        let mut retained: JurisdictionState = serde_json::from_value(payload)?;
        check!(eq; serde_json::to_value(&retained)?, serde_json::to_value(&state)?);
        let plan = retained.plan.as_mut().ok_or("California plan missing")?;
        retained.coach_lane = bind(plan, UsJurisdiction::California, season()?, 1, OBSERVED_ON)?;
        check!(eq; serde_json::to_value(&retained)?, serde_json::to_value(&state)?);
        check!(coach_lane_owed(&retained));
        let refusal = retained
            .coach_lane
            .as_ref()
            .and_then(CoachLaneObligation::refusal)
            .ok_or("California refusal missing")?;
        check!(eq; refusal.reason(), CA_TRUE_MISSING);
        check!(eq; refusal.source().id.as_str(), CA_REFUSAL_SLUG);
        check!(eq; store.journal_keys("ca-coach-qualification")?.len(), 1);
        check!(eq; store.walk_table(Table::Coaches)?.rows, 0);
        Ok(())
    }
}
