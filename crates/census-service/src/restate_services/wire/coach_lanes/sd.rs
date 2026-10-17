#[cfg(test)]
use census_domain::model::{CoachLaneError, CoachLaneEvidence, CoachLaneObligation, CoachLaneResolution, CoachLaneRole, CoachLaneTenure, PendingCoachLane, SchoolYear, COACH_LANE_MAX_FRESHNESS_DAYS};
use census_domain::model::CoachLaneMailbox;
use census_domain::UsJurisdiction;

use super::policy::{ArmBasis, Lane, LanePolicy, RetainedCapture};
use crate::restate_services::plan::RefusalKind;

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

const CAPTURE: RetainedCapture = RetainedCapture {
    schools: SD_SCHOOLS,
    contacts: SD_CONTACTS,
    digest: SD_YANKTON_GIRLS_DIGEST,
    captured_on: SD_CAPTURE_ON,
    source_url: SD_SOURCE_URL,
    mailbox: CoachLaneMailbox::Unpublished,
    note: SD_RESOLUTION_NOTE,
    captures: &SD_CAPTURES,
    stale_path: SD_YANKTON_GIRLS_PATH,
    jurisdiction_name: "South Dakota",
};

pub(super) const POLICY: Lane = Lane {
    jurisdiction: UsJurisdiction::SouthDakota,
    arm: SD_ARM,
    refusal: (SD_REFUSAL_SLUG, SD_TRUE_MISSING),
    kind: RefusalKind::EngineeringGap,
    basis: ArmBasis::Text(SD_ARM_BASIS),
    policy: LanePolicy::HeldCapture(&CAPTURE),
};

#[cfg(test)]
mod tests;
