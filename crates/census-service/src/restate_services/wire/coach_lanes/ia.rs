#[cfg(test)]
use census_domain::model::CoachLaneAcceptance;
use census_domain::model::{
    CoachLaneError, CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution, CoachLaneStage,
    PendingCoachLane, SchoolYear, SourceRef,
};
use census_domain::UsJurisdiction;

pub(super) const IA_ARM: &str = "bound";
pub(super) const IA_REFUSAL_SLUG: &str = "ia_missing_coach_sources";
pub(super) const IA_TRUE_MISSING: &str = "`ighsau.org` (girls union; site uncaptured — only PDFs held) · `gobound.com/ia/{ihsaa,ighsau}` (refused to this client) · `iowarunjumpthrow.com` (IATC assoc; E1 #7; probe seeds held, site uncaptured) · school sites for email (**email 11/1339 is the standout gap**).";

const IA_ARM_BASIS: &str = "the GoBound association staff pages are the only observed Iowa coach-role surface and the host refuses this client, while the held IHSAA and IGHSAU material publishes no coach contacts";
#[cfg(test)]
const ACCEPTED_BY: &str = "census-ops";
#[cfg(test)]
const ACCEPTANCE_EVENT: &str = "jpx4.accepted_refusal";

pub(super) fn refusal(observed_on: &str) -> Result<CoachLaneRefusal, CoachLaneError> {
    CoachLaneRefusal::new(SourceRef::id(IA_REFUSAL_SLUG), IA_TRUE_MISSING, observed_on)
}

pub(super) fn lane(
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    let pending = CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction: UsJurisdiction::Iowa,
        season,
        revision,
        arm: IA_ARM.to_string(),
        arm_basis: IA_ARM_BASIS.to_string(),
        observed_on: observed_on.to_string(),
    })?;
    pending.record(
        CoachLaneStage::Blocked,
        Some(refusal(observed_on)?),
        CoachLaneResolution::unresolved(observed_on),
    )
}

#[cfg(test)]
pub(super) fn accepted(
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    let recorded = lane(season, revision, observed_on)?;
    let acceptance = CoachLaneAcceptance::new(ACCEPTED_BY, ACCEPTANCE_EVENT, observed_on)?;
    recorded.accept_refusal(acceptance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::restate_services::jurisdiction::{coach_lane_owed, owed_work};
    use crate::restate_services::plan::{
        compute_plan_fingerprint, plan as planned, BrowserLaneState, RefusalKind,
    };
    use crate::restate_services::wire::{
        HistoryWindow, JurisdictionRequest, JurisdictionState, SourcePlan,
    };
    use census_domain::model::CoachLaneStatus;
    use census_reconcile::identity::Revision;
    use census_store::{Store, Table};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const OBSERVED_ON: &str = "2026-10-11";
    const WORK_ORDER: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/sources/coach-coverage-bundle-20261004/state-work-orders/group-b.md"
    ));

    fn season() -> Result<SchoolYear, Box<dyn std::error::Error>> {
        Ok(SchoolYear::new(2026).ok_or("school year 2026")?)
    }

    #[test]
    fn coach_lane_ia_records_the_work_order_true_missing_reason() -> TestResult {
        let recorded = refusal(OBSERVED_ON)?;
        check!(eq; recorded.reason(), IA_TRUE_MISSING);
        check!(eq; recorded.source().id.as_str(), IA_REFUSAL_SLUG);
        check!(eq; recorded.observed_on(), OBSERVED_ON);
        check!(
            WORK_ORDER.contains(IA_TRUE_MISSING),
            "the recorded reason is the work order's Iowa true-missing line"
        );
        Ok(())
    }

    #[test]
    fn coach_lane_ia_lane_records_the_refusal_without_closing_the_row() -> TestResult {
        let mut state = JurisdictionState::default();
        check!(coach_lane_owed(&state));
        let row = lane(season()?, 1, OBSERVED_ON)?;
        check!(eq; row.id(), "coach_lane:IA:2026:1");
        check!(eq; row.status(), CoachLaneStatus::Blocked);
        check!(!row.is_terminal());
        check!(eq; row.arm_name(), IA_ARM);
        check!(row.refusal().is_some());
        check!(row.acceptance().is_none());
        check!(row.evidence().is_none());
        state.coach_lane = Some(row);
        check!(coach_lane_owed(&state));
        Ok(())
    }

    #[test]
    fn coach_lane_ia_owner_acceptance_closes_the_row() -> TestResult {
        let mut state = JurisdictionState::default();
        let closed = accepted(season()?, 1, OBSERVED_ON)?;
        check!(eq; closed.status(), CoachLaneStatus::AcceptedRefusal);
        check!(closed.is_terminal());
        check!(closed.evidence().is_none());
        let acceptance = closed
            .acceptance()
            .ok_or("an accepted refusal carries one")?;
        check!(eq; acceptance.actor(), ACCEPTED_BY);
        check!(eq; acceptance.event(), ACCEPTANCE_EVENT);
        check!(eq; acceptance.observed_on(), OBSERVED_ON);
        state.coach_lane = Some(closed);
        check!(!coach_lane_owed(&state));
        Ok(())
    }

    fn request() -> Result<JurisdictionRequest, Box<dyn std::error::Error>> {
        Ok(JurisdictionRequest {
            jurisdiction: UsJurisdiction::Iowa,
            season: season()?,
            revision: Revision(1),
            history: HistoryWindow::cohort(OBSERVED_ON)?,
            refresh: false,
            limit_per_state: None,
            concurrency: 1,
            observed_on: Some(OBSERVED_ON.to_string()),
            authorized_hosts: Vec::new(),
            source_parallelism: 1,
        })
    }

    fn bound_state() -> Result<JurisdictionState, Box<dyn std::error::Error>> {
        let request = request()?;
        let browser = BrowserLaneState::Absent;
        let fingerprint = compute_plan_fingerprint(
            request.jurisdiction,
            request.season,
            request.revision,
            browser,
        );
        let mut plan = SourcePlan::of(&planned(request.jurisdiction, browser), fingerprint);
        let coach_lane = super::super::bind(
            &mut plan,
            request.jurisdiction,
            request.season,
            request.revision.get(),
            OBSERVED_ON,
        )?
        .ok_or("Iowa must have a registered coach lane")?;
        Ok(JurisdictionState {
            identity: "jurisdiction:IA:2026-27:1".to_string(),
            plan: Some(plan),
            coach_lane: Some(coach_lane),
            ..JurisdictionState::default()
        })
    }

    fn assert_refusal(state: &JurisdictionState) -> TestResult {
        let plan = state.plan.as_ref().ok_or("Iowa plan missing")?;
        let entries: Vec<_> = plan
            .refused
            .iter()
            .filter(|entry| entry.slug == IA_REFUSAL_SLUG)
            .collect();
        check!(eq; entries.len(), 1);
        let entry = entries.first().ok_or("Iowa RefusedSource missing")?;
        check!(eq; entry.reason.as_str(), IA_TRUE_MISSING);
        check!(eq; entry.kind, RefusalKind::AccessBlocked);
        let lane = state.coach_lane.as_ref().ok_or("Iowa obligation missing")?;
        check!(eq; lane.id(), "coach_lane:IA:2026:1");
        check!(eq; lane.status(), CoachLaneStatus::Blocked);
        let refusal = lane.refusal().ok_or("Iowa obligation refusal missing")?;
        check!(eq; refusal.reason(), IA_TRUE_MISSING);
        check!(eq; refusal.source().id.as_str(), IA_REFUSAL_SLUG);
        check!(eq; refusal.observed_on(), OBSERVED_ON);
        check!(lane.evidence().is_none());
        check!(lane.acceptance().is_none());
        check!(coach_lane_owed(state));
        let (owed, reasons) = owed_work(state, &request()?);
        check!(owed.iter().any(|stage| stage == "coach_lane"));
        check!(reasons.contains(&format!("refused {IA_REFUSAL_SLUG}: {IA_TRUE_MISSING}")));
        Ok(())
    }

    #[test]
    fn coach_lane_ia_registry_refusal_is_idempotent_and_surfaces_the_exact_reason() -> TestResult {
        let mut state = bound_state()?;
        assert_refusal(&state)?;
        let before = serde_json::to_value(&state)?;
        let plan = state.plan.as_mut().ok_or("Iowa plan missing")?;
        state.coach_lane =
            super::super::bind(plan, UsJurisdiction::Iowa, season()?, 1, OBSERVED_ON)?;
        check!(eq; serde_json::to_value(&state)?, before);
        assert_refusal(&state)
    }

    #[test]
    fn coach_lane_ia_persisted_refusal_survives_reopen_and_remains_visible() -> TestResult {
        let root = tempfile::tempdir()?;
        let state = bound_state()?;
        let payload = serde_json::to_value(&state)?;
        let phase = "ia-coach-refusal-qualification";
        {
            let store = Store::open(root.path())?;
            store.journal_done(phase, &state.identity, &payload)?;
            store.flush()?;
        }
        let store = Store::open(root.path())?;
        let retained = store
            .journal_payload(phase, &state.identity)?
            .ok_or("Iowa refusal lost on reopen")?;
        check!(eq; retained, payload);
        let decoded: JurisdictionState = serde_json::from_value(retained)?;
        assert_refusal(&decoded)?;
        store.journal_done(phase, &decoded.identity, &decoded)?;
        check!(eq; store.journal_keys(phase)?.len(), 1);
        check!(store
            .stats()?
            .tables
            .contains(&(Table::Coaches.file().to_string(), 0)));
        check!(eq; store.journal_payload(phase, &decoded.identity)?, Some(payload));
        Ok(())
    }
}
