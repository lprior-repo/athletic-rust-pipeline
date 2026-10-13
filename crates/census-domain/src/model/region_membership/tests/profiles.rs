use super::*;
use crate::model::{DiscoveryPriorityV1, GradYear, PerformanceWindowV1, ProfilePoliciesV1};
use crate::model::{
    ProvisionalInclusionV1, QueryProfileInputV1, QueryProfileV1, RegionalMembershipV1,
};

fn profile(
    membership: RegionalMembershipV1,
    priority: DiscoveryPriorityV1,
) -> TestResult<QueryProfileV1> {
    let reference = ContractRefV1::new(1, 1, &"c".repeat(64))?;
    Ok(QueryProfileV1::new(QueryProfileInputV1 {
        version: 1,
        graduation_years: vec![GradYear::CO2027],
        jurisdictions: vec![
            UsJurisdiction::Michigan,
            UsJurisdiction::Minnesota,
            UsJurisdiction::Iowa,
            UsJurisdiction::Illinois,
            UsJurisdiction::Colorado,
        ],
        regional_membership: membership,
        discovery_priority: priority,
        sports: vec![Sport::CrossCountry],
        competition_categories: vec![Gender::Boys, Gender::Girls],
        as_of: date("2026-10-01")?,
        performance_window: PerformanceWindowV1::AllRetainedThroughAsOf,
        contact_school_year: xc_boys()?.school_year,
        source_plan: reference.clone(),
        acquisition_window: range("2026-08-01", "2026-10-13")?,
        completion_contract: reference.clone(),
        policies: ProfilePoliciesV1 {
            cohort: reference.clone(),
            event: reference.clone(),
            identity: reference.clone(),
            affiliation: reference.clone(),
            contact: reference.clone(),
            participation: reference.clone(),
            publication: reference,
        },
        include_provisional: ProvisionalInclusionV1::Exclude,
    })?)
}

#[test]
fn up_first_priority_preserves_full_five_state_membership_and_completion_scope() -> TestResult {
    let a = school("a", UsJurisdiction::Michigan)?;
    let b = school("b", UsJurisdiction::Michigan)?;
    let payload = contract(vec![
        assertion(
            RegionSubjectV1::School(a.clone()),
            RegionStatementV1::Member,
            "up",
        )?,
        assertion(
            RegionSubjectV1::School(b.clone()),
            RegionStatementV1::ExplicitNonMember,
            "lp",
        )?,
    ])?;
    let reference = payload.reference()?;
    let uniform = profile(
        RegionalMembershipV1::AllSelectedJurisdictions,
        DiscoveryPriorityV1::Uniform,
    )?;
    let first = profile(
        RegionalMembershipV1::AllSelectedJurisdictions,
        DiscoveryPriorityV1::SourceBackedRegionFirst(reference.clone()),
    )?;
    let only = profile(
        RegionalMembershipV1::SourceBackedRegion(reference),
        DiscoveryPriorityV1::Uniform,
    )?;
    check!(eq; first.input().jurisdictions, uniform.input().jurisdictions);
    check!(eq; first.input().completion_contract, uniform.input().completion_contract);
    check!(matches!(
        &first.input().regional_membership,
        RegionalMembershipV1::AllSelectedJurisdictions
    ));
    check!(matches!(
        &only.input().regional_membership,
        RegionalMembershipV1::SourceBackedRegion(_)
    ));
    check!(ne; first.id()?, only.id()?);
    for (school, expected) in [
        (a, RegionPriorityTierV1::RegionFirst),
        (b, RegionPriorityTierV1::Standard),
    ] {
        let query = query(RegionMembershipTargetV1::School(school), "2026-10-01")?;
        check!(eq; resolve_region_priority_v1(&payload, &query).tier, expected);
    }
    Ok(())
}

#[test]
fn reference_binding_refuses_wrong_revision_or_semantic_payload() -> TestResult {
    let a = school("a", UsJurisdiction::Michigan)?;
    let payload = contract(vec![assertion(
        RegionSubjectV1::School(a),
        RegionStatementV1::Member,
        "member",
    )?])?;
    let reference = payload.reference()?;
    check!(eq; bind_region_contract_v1(&reference, &payload)?.contract(), &payload);
    let wrong_revision = ContractRefV1::new(1, 8, reference.digest())?;
    check!(matches!(
        bind_region_contract_v1(&wrong_revision, &payload),
        Err(RegionMembershipError::ReferenceMismatch)
    ));
    let wrong_digest = ContractRefV1::new(1, 7, &"0".repeat(64))?;
    check!(matches!(
        bind_region_contract_v1(&wrong_digest, &payload),
        Err(RegionMembershipError::ReferenceMismatch)
    ));
    let mut changed = payload.input().clone();
    changed.effective = range("2026-01-02", "2027-12-31")?;
    check!(matches!(
        bind_region_contract_v1(&reference, &RegionMembershipContractV1::new(changed)?),
        Err(RegionMembershipError::ReferenceMismatch)
    ));
    Ok(())
}

#[test]
fn unresolved_priority_does_not_become_standard_or_change_membership() -> TestResult {
    let payload = contract(vec![])?;
    let a = school("a", UsJurisdiction::Michigan)?;
    let query = query(RegionMembershipTargetV1::School(a), "2026-10-01")?;
    let result = resolve_region_priority_v1(&payload, &query);
    check!(eq; result.tier, RegionPriorityTierV1::Unresolved);
    check!(eq; result.membership_evidence.verdict(), RegionMembershipVerdictV1::Unknown);
    Ok(())
}
