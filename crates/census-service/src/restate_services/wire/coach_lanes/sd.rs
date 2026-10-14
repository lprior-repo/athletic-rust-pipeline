use census_domain::model::{
    CoachLaneError, CoachLaneEvidence, CoachLaneMailbox, CoachLaneObligation, CoachLaneRefusal,
    CoachLaneResolution, CoachLaneRole, CoachLaneStage, CoachLaneTenure, PendingCoachLane,
    SchoolYear, SourceRef, COACH_LANE_MAX_FRESHNESS_DAYS,
};
use census_domain::UsJurisdiction;

use super::freshness_days;

pub(super) const SD_ARM: &str = "bound";
pub(super) const SD_REFUSAL_SLUG: &str = "sd_missing_coach_sources";
pub(super) const SD_TRUE_MISSING: &str = "`www.gobound.com/sd/sdhsaa/...` staff pages for schools not among the 560 held rows (gobound live was refused/listed as blocked) · SDHSAA member directory itself (SRC-224) — not held; school-side directory (SD DOE naming) · School district sites for coach emails (SDHSAA publishes none)";
pub(super) const SD_YANKTON_GIRLS_PATH: &str =
    "crates/census-crawl/tests/fixtures/bound/staff-yankton-sd-girls-xc.html";
pub(super) const SD_YANKTON_GIRLS_DIGEST: &str =
    "3bdab3bd25bbc3ae5ff06c308922b7bd8480a303877abbfcd3aecf32cd5dc93f";
pub(super) const SD_YANKTON_BOYS_PATH: &str =
    "crates/census-crawl/tests/fixtures/bound/staff-yankton-sd-boys-xc.html";
pub(super) const SD_YANKTON_BOYS_DIGEST: &str =
    "55f0a5027ed504c15a2003c8a0eb3de48cee35cff9f615c05ca9972f9d076ccc";
pub(super) const SD_WHITERIVER_PATH: &str =
    "crates/census-crawl/tests/fixtures/bound/staff-whiteriver-sd-girls-xc.html";
pub(super) const SD_WHITERIVER_DIGEST: &str =
    "d8d0353e15399fae06426ff15d7ed27834b59be0f255003bc1d3eb42967064f5";
pub(super) const SD_FAITH_PATH: &str =
    "crates/census-crawl/tests/fixtures/bound/staff-faith-sd-boys-xc-empty-staff.html";
pub(super) const SD_FAITH_DIGEST: &str =
    "c4a3d7b76a7d4bebd45ca04e039d6071b7f26f35191fed845ee57d37c76efcf2";
pub(super) const SD_CAPTURES: [(&str, &str); 4] = [
    (SD_YANKTON_GIRLS_PATH, SD_YANKTON_GIRLS_DIGEST),
    (SD_YANKTON_BOYS_PATH, SD_YANKTON_BOYS_DIGEST),
    (SD_WHITERIVER_PATH, SD_WHITERIVER_DIGEST),
    (SD_FAITH_PATH, SD_FAITH_DIGEST),
];
pub(super) const SD_SOURCE_URL: &str =
    "https://www.gobound.com/sd/sdhsaa/girlscrosscountry/2026-27/yankton/v/staff";
pub(super) const SD_SCHOOLS: u32 = 3;
pub(super) const SD_CONTACTS: u32 = 6;
pub(super) const SD_CAPTURE_ON: &str = "2026-10-07";

const SD_ARM_BASIS: &str = "the bound descriptor declares school-coach contact and is admitted for South Dakota, and TeamsArm::Bound walks each SDHSAA school's GoBound staff pages; the retained SD staff captures are the held output of that walk";
const SD_RESOLUTION_NOTE: &str = "four retained 2026-27-season GoBound SDHSAA staff captures (Yankton boys and girls cross country, White River girls cross country, Faith boys cross country - the retained empty-staff edge) list Head Coach and Assistant Coach rows and publish no mailbox; the crawl cache records no fetch timestamp, so 2026-10-07 is their documented offline replay date (research/sources/bound/SOURCE_REPORT.md):";

fn resolved_note() -> String {
    let mut note = String::from(SD_RESOLUTION_NOTE);
    for (path, digest) in SD_CAPTURES {
        note.push_str(&format!(" {path} sha256 {digest};"));
    }
    note
}

pub(super) fn lane(
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    let pending = CoachLaneObligation::pending(PendingCoachLane {
        jurisdiction: UsJurisdiction::SouthDakota,
        season,
        revision,
        arm: SD_ARM.to_string(),
        arm_basis: SD_ARM_BASIS.to_string(),
        observed_on: observed_on.to_string(),
    })?;
    if let Some(reason) = census_crawl::registry::acquisition_refusal_for_source(SD_ARM) {
        return pending.record(
            CoachLaneStage::Blocked,
            Some(CoachLaneRefusal::new(
                SourceRef::id(SD_ARM),
                reason,
                observed_on,
            )?),
            CoachLaneResolution::unresolved(observed_on),
        );
    }
    let freshness = freshness_days(SD_CAPTURE_ON, observed_on)?;
    if freshness > COACH_LANE_MAX_FRESHNESS_DAYS {
        return pending.record(
            CoachLaneStage::Blocked,
            Some(stale_refusal(freshness, observed_on)?),
            CoachLaneResolution::unresolved(observed_on),
        );
    }
    let evidence = CoachLaneEvidence::new(
        SD_SCHOOLS,
        SD_CONTACTS,
        SD_YANKTON_GIRLS_DIGEST,
        SD_CAPTURE_ON,
    )?
    .with_source_url(SD_SOURCE_URL)?;
    let resolution = CoachLaneResolution::resolved(
        CoachLaneRole::Published,
        CoachLaneTenure::Current,
        CoachLaneMailbox::Unpublished,
        SD_CAPTURE_ON,
        freshness,
        resolved_note(),
    )?;
    pending.arm(evidence, resolution)
}

fn stale_refusal(freshness: u32, observed_on: &str) -> Result<CoachLaneRefusal, CoachLaneError> {
    CoachLaneRefusal::new(
        SourceRef::id(SD_ARM),
        format!(
            "the held bound capture {SD_YANKTON_GIRLS_PATH} is {freshness} days old at lane creation, beyond the {COACH_LANE_MAX_FRESHNESS_DAYS}-day bound, so the South Dakota arm cannot retain it"
        ),
        observed_on,
    )
}

#[cfg(test)]
mod tests;
