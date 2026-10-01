mod data;
mod prose;

pub(super) use data::TABLE;

const ARBITER: super::Applicability = super::Applicability {
    slug: "arbiter_orgs",
    jurisdictions: &census_domain::UsJurisdiction::CENSUS_SCOPE,
    evidence: "Arbiter Sports publishes a national coach directory accessible via structured API.",
    refusal: "None - the directory is nationally scoped.",
};
