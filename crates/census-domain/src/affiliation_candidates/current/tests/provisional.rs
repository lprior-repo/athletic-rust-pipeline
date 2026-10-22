use super::*;
use crate::model::serialized_bytes;

fn include_profile(profile: &QueryProfileV1) -> TestResult<QueryProfileV1> {
    let mut raw = profile.input().clone();
    raw.include_provisional = ProvisionalInclusionV1::Include;
    Ok(QueryProfileV1::new(raw)?)
}

#[test]
fn explicit_opt_in_preserves_the_same_checked_affiliation_facts() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let published = [&fixture.owner];
    let assertions = [LocallyCheckedAssertion::check(fixture.assertion(
        &evidence,
        &published,
        key("claim-A")?,
    )?)?];
    let exclude = profile("2026-10-10")?;
    let include = include_profile(&exclude)?;
    let exclude_bytes = serialized_bytes(&exclude)?;
    let include_bytes = serialized_bytes(&include)?;
    let exclude_request = request(&fixture, &exclude)?;
    let include_request = request(&fixture, &include)?;
    let excluded = assess_current_affiliation(
        exclude_request,
        inventory(exclude_request, &assertions, &[]),
    )?;
    let included = assess_current_affiliation(
        include_request,
        inventory(include_request, &assertions, &[]),
    )?;
    check!(eq; excluded.outcome, CurrentAffiliationOutcome::ResolvedSchoolCandidate(&fixture.school));
    check!(eq; included.outcome, excluded.outcome);
    check!(eq; &included.facts, &excluded.facts);
    check!(eq; fact_ids(&included), fact_ids(&excluded));
    check!(eq; &included.profile_id, &include.id()?);
    check!(eq; &excluded.profile_id, &exclude.id()?);
    check!(ne; include.id()?, exclude.id()?);
    check!(eq; serialized_bytes(&exclude)?, exclude_bytes);
    check!(eq; serialized_bytes(&include)?, include_bytes);
    Ok(())
}

#[test]
fn explicit_opt_in_cannot_create_current_affiliation_without_facts() -> TestResult {
    let fixture = Fixture::new()?;
    let exclude = profile("2026-10-10")?;
    let include = include_profile(&exclude)?;
    for profile in [&exclude, &include] {
        let request = request(&fixture, profile)?;
        let assessed = assess_current_affiliation(request, inventory(request, &[], &[]))?;
        check!(eq; assessed.outcome, CurrentAffiliationOutcome::Unknown);
        check!(assessed.facts.is_empty());
        check!(eq; &assessed.profile_id, &profile.id()?);
    }
    Ok(())
}
