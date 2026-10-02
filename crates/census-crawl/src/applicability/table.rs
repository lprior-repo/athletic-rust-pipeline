mod data;
mod prose;

pub(super) use data::TABLE;

const ARBITER: super::Applicability = super::Applicability {
    slug: "arbiter_orgs",
    jurisdictions: &[
        census_domain::UsJurisdiction::Kentucky,
        census_domain::UsJurisdiction::Montana,
        census_domain::UsJurisdiction::NewHampshire,
        census_domain::UsJurisdiction::WestVirginia,
    ],
    evidence: "Association-delegated Arbiter member and coach directories are registered for \
               NHIAA org 2132, Kentucky org 2507, Montana org 4497 and West Virginia org 4223.",
    refusal: "Only these four supported organisations are registered; other jurisdictions have \
              no qualified organisation in this collector. Token or source refusal remains \
              explicit and does not establish an empty directory.",
};
