use super::*;
use crate::affiliation_candidates::*;
use crate::model::{
    DiscoveryPriorityV1, Gender, GradYear, PerformanceWindowV1, ProfileDateRangeV1,
    ProfilePoliciesV1, ProvisionalInclusionV1, QueryProfileInputV1, RegionalMembershipV1,
    SchoolYear,
};
use crate::profile_execution::{profile_contract_ref_v1, ContractRoleV1};
use crate::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod boundaries;
#[path = "../tests/fixture.rs"]
mod fixture;
mod metadata;
mod relationships;
mod retained;
mod transfer;

use fixture::{day, grant, instant, key, known, Fixture};

fn profile(as_of: &str) -> TestResult<QueryProfileV1> {
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![UsJurisdiction::Michigan],
        regional_membership: RegionalMembershipV1::AllSelectedJurisdictions,
        discovery_priority: DiscoveryPriorityV1::Uniform,
        sports: vec![Sport::CrossCountry],
        competition_categories: vec![Gender::Boys, Gender::Girls],
        as_of: day(as_of)?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: SchoolYear::new(2026).ok_or("synthetic school year")?,
        source_plan: profile_contract_ref_v1(ContractRoleV1::SourcePlan)?,
        acquisition_window: ProfileDateRangeV1::new(day("2026-08-01")?, day("2026-10-31")?)?,
        completion_contract: profile_contract_ref_v1(ContractRoleV1::Completion)?,
        policies: ProfilePoliciesV1 {
            cohort: profile_contract_ref_v1(ContractRoleV1::Cohort)?,
            event: profile_contract_ref_v1(ContractRoleV1::Event)?,
            identity: profile_contract_ref_v1(ContractRoleV1::Identity)?,
            affiliation: profile_contract_ref_v1(ContractRoleV1::Affiliation)?,
            contact: profile_contract_ref_v1(ContractRoleV1::Contact)?,
            participation: profile_contract_ref_v1(ContractRoleV1::Participation)?,
            publication: profile_contract_ref_v1(ContractRoleV1::Publication)?,
        },
        include_provisional: ProvisionalInclusionV1::Exclude,
    })?)
}

fn request<'a>(
    fixture: &'a Fixture,
    profile: &'a QueryProfileV1,
) -> TestResult<CurrentAffiliationRequest<'a>> {
    let legacy = fixture.query()?;
    Ok(CurrentAffiliationRequest {
        athlete: &fixture.athlete,
        profile,
        sport: Sport::CrossCountry,
        known: legacy.known,
        acquisition_cutoff: legacy.acquisition_cutoff,
        grant: grant()?,
    })
}

fn inventory<'a>(
    request: CurrentAffiliationRequest<'a>,
    assertions: &'a [LocallyCheckedAssertion<'a>],
    changes: &'a [LocallyCheckedChange<'a>],
) -> CurrentAffiliationInventory<'a> {
    CurrentAffiliationInventory {
        declared_view: request.known,
        assertions,
        changes,
    }
}

fn fact_ids<'a>(assessment: &CurrentAffiliationAssessment<'a>) -> Vec<ObjectRef<'a>> {
    assessment
        .facts
        .iter()
        .map(|fact| match fact {
            FactDisposition::Assertion { fact, .. } => fact.raw().id,
            FactDisposition::Change { fact, .. } => fact.raw().id,
        })
        .collect()
}
