pub(super) mod ct;
pub(super) mod ia;
pub(super) mod ma;
pub(super) mod mo;
pub(super) mod nv;
pub(super) mod ok;
pub(super) mod or;
pub(super) mod sd;
pub(super) mod ut;
pub(super) mod vt;

pub(super) mod ca;
pub(super) mod co;
pub(super) mod fl;
pub(super) mod il;
pub(super) mod ny;
pub(super) mod tx;

pub(super) mod az;
pub(super) mod indiana;
pub(super) mod la;
pub(super) mod mi;
pub(super) mod nj;
pub(super) mod va;
pub(super) mod wa;

mod missing;

use census_domain::model::{
    CoachLaneError, CoachLaneObligation, CoachLaneStatus, SchoolYear,
};
use census_domain::UsJurisdiction;
use chrono::NaiveDate;

use crate::restate_services::wire::{RefusedSource, SourcePlan};

mod policy;
use policy::{Acquisition, Lane, LanePolicy};

pub(in crate::restate_services) fn refusal_slug(
    jurisdiction: UsJurisdiction,
) -> Option<&'static str> {
    descriptor(jurisdiction).map(|row| row.refusal.0)
}

fn descriptor(jurisdiction: UsJurisdiction) -> Option<&'static Lane> {
    LANES.iter().copied().chain(missing::LANES.iter())
        .find(|row| row.jurisdiction == jurisdiction)
}

const LANES: &[&Lane] = &[
    &az::POLICY,
    &ca::POLICY,
    &co::POLICY,
    &ct::POLICY,
    &fl::POLICY,
    &il::POLICY,
    &indiana::POLICY,
    &ia::POLICY,
    &la::POLICY,
    &ma::POLICY,
    &mi::POLICY,
    &mo::POLICY,
    &nv::POLICY,
    &nj::POLICY,
    &ny::POLICY,
    &ok::POLICY,
    &or::POLICY,
    &sd::POLICY,
    &tx::POLICY,
    &ut::POLICY,
    &vt::POLICY,
    &va::POLICY,
    &wa::POLICY,
];

fn evaluate(
    row: &Lane,
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    let acquisition = match row.policy {
        LanePolicy::HeldCapture(_) => match census_crawl::registry::acquisition_refusal_for_source(row.arm) {
            Some(reason) => Acquisition::Held(reason),
            None => Acquisition::Available,
        },
        _ => Acquisition::Available,
    };
    row.evaluate(season, revision, observed_on, acquisition)
}

#[cfg(test)]
fn accepted(
    row: &Lane,
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<CoachLaneObligation, CoachLaneError> {
    let recorded = evaluate(row, season, revision, observed_on)?;
    let acceptance = census_domain::model::CoachLaneAcceptance::new(
        "census-ops", "jpx4.accepted_refusal", observed_on,
    )?;
    recorded.accept_refusal(acceptance)
}

pub(in crate::restate_services) fn freshness_days(
    capture_on: &str,
    observed_on: &str,
) -> Result<u32, CoachLaneError> {
    let capture = NaiveDate::parse_from_str(capture_on, "%Y-%m-%d")
        .map_err(|_| out_of_range("resolution.retrieved_on"))?;
    let observed = NaiveDate::parse_from_str(observed_on, "%Y-%m-%d")
        .map_err(|_| out_of_range("updated_on"))?;
    let days = observed.signed_duration_since(capture).num_days();
    u32::try_from(days).map_err(|_| out_of_range("resolution.freshness_days"))
}

pub(in crate::restate_services) fn bind(
    plan: &mut SourcePlan,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    revision: u32,
    observed_on: &str,
) -> Result<Option<CoachLaneObligation>, CoachLaneError> {
    let Some(row) = descriptor(jurisdiction) else {
        return Ok(None);
    };
    let obligation = evaluate(row, season, revision, observed_on)?;
    let armed = obligation.status() == CoachLaneStatus::Armed;
    if armed && !plan.sweepable.iter().any(|slug| slug.as_str() == row.arm) {
        return Err(CoachLaneError::OutOfRange { field: "arm" });
    }
    let recorded = row.refusal(observed_on)?;
    let already_recorded = plan
        .refused
        .iter()
        .any(|entry| entry.slug == recorded.source().id);
    if !already_recorded {
        plan.refused.push(RefusedSource {
            slug: recorded.source().id.to_string(),
            reason: recorded.reason().to_string(),
            kind: row.kind,
        });
    }
    Ok(Some(obligation))
}

fn out_of_range(field: &'static str) -> CoachLaneError {
    CoachLaneError::OutOfRange { field }
}

#[cfg(test)]
pub(super) fn capture_digest(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = std::fs::read(root.join(path))?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

#[cfg(test)]
#[path = "coach_lanes/registry_tests.rs"]
mod registry_tests;

#[cfg(test)]
#[path = "coach_lanes/policy_tests.rs"]
mod policy_tests;
