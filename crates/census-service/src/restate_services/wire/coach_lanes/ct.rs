use census_domain::model::{
    CoachLaneError, CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution, CoachLaneStage,
    PendingCoachLane, SchoolYear, SourceRef,
};
use census_domain::UsJurisdiction;

pub(super) const CT_ARM: &str = "ciac";
pub(super) const CT_REFUSAL_SLUG: &str = "ct_missing_coach_sources";
pub(super) const CT_TRUE_MISSING: &str = "`chsca.org/officers.html` (CT coaches assoc XC/track committees; SRC-080; unheld) · `fciac.net` 2021-22 coach directory PDF (SRC-081; historical, unheld) · `public-edsight.ct.gov` (SRC-078/079) · `nepsac.org` (SRC-093) · school sites for **email (12/1021)**.";
pub(super) const CT_CAPTURE_PATH: &str = "research/sources/coach-coverage-bundle-20261004/probes/ciac-fpsports/raw/directory-SchoolLevelID-1.adapter.body";
pub(super) const CT_CAPTURE_DIGEST: &str =
    "a872654b3c9450a484c6f68ae75f9bd32c2523af0f7a2c118f8bb823f1cfc634";
pub(super) const CT_SOURCE_URL: &str = "https://ciac.fpsports.org/Directory.aspx?SchoolLevelID=1";
#[cfg(test)]
pub(super) const CT_CAPTURE_BYTES: u64 = 673_123;

#[cfg(test)]
const CT_CAPTURE_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/sources/coach-coverage-bundle-20261004/probes/ciac-fpsports/raw/directory-SchoolLevelID-1.adapter.body"
);
const CT_ARM_BASIS: &str = "the registered ciac arm parses the 2026-10-04 FusionPoint directory into coach-name and role rows with phone numbers, not coach mailboxes or page-bound current-tenure evidence, so the held capture cannot arm the lane:";

pub(super) fn refusal(observed_on: &str) -> Result<CoachLaneRefusal, CoachLaneError> {
    CoachLaneRefusal::new(SourceRef::id(CT_REFUSAL_SLUG), CT_TRUE_MISSING, observed_on)
}

pub(super) fn lane(
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    let pending = CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction: UsJurisdiction::Connecticut,
        season,
        revision,
        arm: CT_ARM.to_string(),
        arm_basis: format!(
            "{CT_ARM_BASIS} {CT_SOURCE_URL} {CT_CAPTURE_PATH} sha256 {CT_CAPTURE_DIGEST}"
        ),
        observed_on: observed_on.to_string(),
    })?;
    pending.record(
        CoachLaneStage::Blocked,
        Some(refusal(observed_on)?),
        CoachLaneResolution::unresolved(observed_on),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::restate_services::jurisdiction::coach_lane_owed;
    use crate::restate_services::teams_arms;
    use crate::restate_services::wire::JurisdictionState;
    use census_domain::model::CoachLaneStatus;
    use sha2::{Digest, Sha256};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const OBSERVED_ON: &str = "2026-10-11";
    const BOUNDARY_ON: &str = "2027-11-08";
    const STALE_ON: &str = "2027-11-09";
    const CIAC_MANIFEST: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/sources/coach-coverage-bundle-20261004/probes/ciac-fpsports/manifest.json"
    ));
    const WORK_ORDER: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../research/sources/coach-coverage-bundle-20261004/state-work-orders/group-b.md"
    ));

    fn season() -> Result<SchoolYear, Box<dyn std::error::Error>> {
        Ok(SchoolYear::new(2026).ok_or("school year 2026")?)
    }

    #[test]
    fn coach_lane_ct_records_the_phone_only_refusal_without_closing() -> TestResult {
        let recorded = lane(season()?, 1, OBSERVED_ON)?;
        check!(eq; recorded.id(), "coach_lane:CT:2026:1");
        check!(eq; recorded.status(), CoachLaneStatus::Blocked);
        check!(!recorded.is_terminal());
        check!(eq; recorded.arm_name(), CT_ARM);
        check!(recorded.evidence().is_none());
        check!(recorded.acceptance().is_none());
        let refusal = recorded
            .refusal()
            .ok_or("phone-only evidence remains refused")?;
        check!(eq; refusal.source().id.as_str(), CT_REFUSAL_SLUG);
        check!(eq; refusal.reason(), CT_TRUE_MISSING);
        check!(recorded.arm_basis().contains(CT_CAPTURE_PATH));
        check!(recorded.arm_basis().contains(CT_CAPTURE_DIGEST));
        let state = JurisdictionState {
            coach_lane: Some(recorded),
            ..JurisdictionState::default()
        };
        check!(coach_lane_owed(&state));
        Ok(())
    }

    #[test]
    fn coach_lane_ct_evidence_digest_is_the_sha256_of_the_capture() -> TestResult {
        let bytes = std::fs::read(CT_CAPTURE_FILE)?;
        check!(eq; u64::try_from(bytes.len())?, CT_CAPTURE_BYTES);
        let digest = format!("{:x}", Sha256::digest(&bytes));
        check!(eq; digest.as_str(), CT_CAPTURE_DIGEST);
        Ok(())
    }

    #[test]
    fn coach_lane_ct_held_directory_agrees_with_the_probe_manifest() -> TestResult {
        check!(CIAC_MANIFEST.contains(CT_CAPTURE_DIGEST));
        check!(CIAC_MANIFEST.contains("raw/directory-SchoolLevelID-1.adapter.body"));
        check!(CIAC_MANIFEST.contains(CT_SOURCE_URL));
        check!(CIAC_MANIFEST.contains("184 schools"));
        check!(CIAC_MANIFEST.contains("coach rows"));
        check!(CIAC_MANIFEST.contains("2026-10-04"));
        Ok(())
    }

    #[test]
    fn coach_lane_ct_arm_is_wired_in_the_teams_arms_table() -> TestResult {
        check!(teams_arms::arm_for(CT_ARM).is_some());
        Ok(())
    }

    #[test]
    fn coach_lane_ct_true_missing_reason_is_the_work_order_line() -> TestResult {
        let recorded = refusal(OBSERVED_ON)?;
        check!(eq; recorded.reason(), CT_TRUE_MISSING);
        check!(eq; recorded.source().id.as_str(), CT_REFUSAL_SLUG);
        check!(WORK_ORDER.contains(CT_TRUE_MISSING));
        Ok(())
    }

    #[test]
    fn coach_lane_ct_capture_age_cannot_supply_missing_contact_evidence() -> TestResult {
        for observed_on in [OBSERVED_ON, BOUNDARY_ON, STALE_ON] {
            let recorded = lane(season()?, 1, observed_on)?;
            check!(eq; recorded.status(), CoachLaneStatus::Blocked);
            check!(!recorded.is_terminal());
            check!(recorded.evidence().is_none());
            let refusal = recorded
                .refusal()
                .ok_or("phone-only evidence remains refused")?;
            check!(eq; refusal.reason(), CT_TRUE_MISSING);
        }
        Ok(())
    }
}
