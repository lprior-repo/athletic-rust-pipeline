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
    CoachLaneError, CoachLaneObligation, CoachLaneRefusal, CoachLaneStatus, SchoolYear, SourceRef,
};
use census_domain::UsJurisdiction;
use chrono::NaiveDate;

use crate::restate_services::plan::RefusalKind;
use crate::restate_services::wire::{RefusedSource, SourcePlan};

pub(super) struct Lane {
    pub(super) jurisdiction: UsJurisdiction,
    pub(super) arm: &'static str,
    pub(super) refusal: (&'static str, &'static str),
    pub(super) kind: RefusalKind,
    pub(super) lane: fn(SchoolYear, u32, &str) -> Result<CoachLaneObligation, CoachLaneError>,
}

pub(in crate::restate_services) fn refusal_slug(
    jurisdiction: UsJurisdiction,
) -> Option<&'static str> {
    if let Some(row) = LANES.iter().find(|row| row.jurisdiction == jurisdiction) {
        return Some(row.refusal.0);
    }
    missing::LANES
        .iter()
        .find(|row| row.jurisdiction == jurisdiction)
        .map(|row| row.slug)
}

pub(super) const LANES: &[Lane] = &[
    Lane {
        jurisdiction: UsJurisdiction::Arizona,
        arm: az::AZ_ARM,
        refusal: (az::AZ_REFUSAL_SLUG, az::AZ_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: az::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::California,
        arm: ca::CA_ARM,
        refusal: (ca::CA_REFUSAL_SLUG, ca::CA_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: ca::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Connecticut,
        arm: ct::CT_ARM,
        refusal: (ct::CT_REFUSAL_SLUG, ct::CT_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: ct::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Florida,
        arm: fl::FL_ARM,
        refusal: (fl::FL_REFUSAL_SLUG, fl::FL_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: fl::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Illinois,
        arm: il::IL_ARM,
        refusal: (il::IL_REFUSAL_SLUG, il::IL_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: il::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Indiana,
        arm: indiana::IN_ARM,
        refusal: (indiana::IN_REFUSAL_SLUG, indiana::IN_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: indiana::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Iowa,
        arm: ia::IA_ARM,
        refusal: (ia::IA_REFUSAL_SLUG, ia::IA_TRUE_MISSING),
        kind: RefusalKind::AccessBlocked,
        lane: ia::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Louisiana,
        arm: la::LA_ARM,
        refusal: (la::LA_REFUSAL_SLUG, la::LA_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: la::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Massachusetts,
        arm: ma::MA_ARM,
        refusal: (ma::MA_REFUSAL_SLUG, ma::MA_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: ma::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Michigan,
        arm: mi::MI_ARM,
        refusal: (mi::MI_REFUSAL_SLUG, mi::MI_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: mi::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Missouri,
        arm: mo::MO_ARM,
        refusal: (mo::MO_REFUSAL_SLUG, mo::MO_TRUE_MISSING),
        kind: RefusalKind::AccessBlocked,
        lane: mo::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Nevada,
        arm: nv::NV_ARM,
        refusal: (nv::NV_REFUSAL_SLUG, nv::NV_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: nv::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::NewJersey,
        arm: nj::NJ_ARM,
        refusal: (nj::NJ_REFUSAL_SLUG, nj::NJ_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: nj::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::NewYork,
        arm: ny::NY_ARM,
        refusal: (ny::NY_REFUSAL_SLUG, ny::NY_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: ny::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Oklahoma,
        arm: ok::OK_ARM,
        refusal: (ok::OK_REFUSAL_SLUG, ok::OK_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: ok::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Oregon,
        arm: or::OR_ARM,
        refusal: (or::OR_REFUSAL_SLUG, or::OR_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: or::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::SouthDakota,
        arm: sd::SD_ARM,
        refusal: (sd::SD_REFUSAL_SLUG, sd::SD_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: sd::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Texas,
        arm: tx::TX_ARM,
        refusal: (tx::TX_REFUSAL_SLUG, tx::TX_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: tx::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Utah,
        arm: ut::UT_ARM,
        refusal: (ut::UT_REFUSAL_SLUG, ut::UT_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: ut::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Vermont,
        arm: vt::VT_ARM,
        refusal: (vt::VT_REFUSAL_SLUG, vt::VT_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: vt::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Virginia,
        arm: va::VA_ARM,
        refusal: (va::VA_REFUSAL_SLUG, va::VA_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: va::lane,
    },
    Lane {
        jurisdiction: UsJurisdiction::Washington,
        arm: wa::WA_ARM,
        refusal: (wa::WA_REFUSAL_SLUG, wa::WA_TRUE_MISSING),
        kind: RefusalKind::EngineeringGap,
        lane: wa::lane,
    },
];

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
    let Some(row) = LANES.iter().find(|row| row.jurisdiction == jurisdiction) else {
        return missing::bind(plan, jurisdiction, season, revision, observed_on);
    };
    let obligation = (row.lane)(season, revision, observed_on)?;
    let armed = obligation.status() == CoachLaneStatus::Armed;
    if armed && !plan.sweepable.iter().any(|slug| slug.as_str() == row.arm) {
        return Err(CoachLaneError::OutOfRange { field: "arm" });
    }
    let (slug, reason) = row.refusal;
    let recorded = CoachLaneRefusal::new(SourceRef::id(slug), reason, observed_on)?;
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
