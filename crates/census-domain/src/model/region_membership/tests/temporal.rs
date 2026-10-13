use super::*;

#[test]
fn regional_membership_uses_exact_inclusive_effective_days() -> TestResult {
    let school = school("a", UsJurisdiction::Michigan)?;
    let payload = contract(vec![assertion(
        RegionSubjectV1::School(school.clone()),
        RegionStatementV1::Member,
        "member",
    )?])?;
    for (effective_on, expected) in [
        ("2026-07-31", RegionMembershipVerdictV1::Unknown),
        ("2026-08-01", RegionMembershipVerdictV1::Member),
        ("2027-07-31", RegionMembershipVerdictV1::Member),
        ("2027-08-01", RegionMembershipVerdictV1::Unknown),
        ("2028-01-01", RegionMembershipVerdictV1::Unknown),
    ] {
        let query = query(
            RegionMembershipTargetV1::School(school.clone()),
            effective_on,
        )?;
        check!(eq; resolve_region_membership_v1(&payload, &query).verdict(), expected);
    }
    Ok(())
}

#[test]
fn undated_negative_absence_and_unknown_classifications_are_not_exclusion() -> TestResult {
    let school = school("a", UsJurisdiction::Michigan)?;
    let target = RegionMembershipTargetV1::School(school.clone());
    let query = query(target, "2026-10-01")?;
    check!(eq; resolve_region_membership_v1(&contract(vec![])?, &query).verdict(), RegionMembershipVerdictV1::Unknown);
    let fact = assertion(
        RegionSubjectV1::School(school.clone()),
        RegionStatementV1::ExplicitNonMember,
        "negative",
    )?;
    let mut undated = fact.input().clone();
    undated.effective = RegionEffectiveV1::Undated;
    let payload = contract(vec![RegionAssertionV1::new(undated)])?;
    let result = resolve_region_membership_v1(&payload, &query);
    check!(eq; result.verdict(), RegionMembershipVerdictV1::Unknown);
    check!(result
        .trace()
        .all(|trace| trace.disposition == RegionAssertionDispositionV1::Undated));
    for unknown_context in [
        context(Sport::Unknown, Gender::Boys, 2026)?,
        context(Sport::CrossCountry, Gender::Unknown, 2026)?,
    ] {
        let payload = contract(vec![replace_assertion_context(&fact, unknown_context)?])?;
        let result = resolve_region_membership_v1(&payload, &query);
        check!(eq; result.verdict(), RegionMembershipVerdictV1::Unknown);
        check!(eq; result.trace().len(), 1);
    }
    Ok(())
}

#[test]
fn exact_sport_category_and_season_do_not_widen_membership() -> TestResult {
    let school = school("a", UsJurisdiction::Michigan)?;
    let fact = assertion(
        RegionSubjectV1::School(school.clone()),
        RegionStatementV1::Member,
        "member",
    )?;
    let query = query(RegionMembershipTargetV1::School(school), "2026-10-01")?;
    for (context, disposition) in [
        (
            context(Sport::OutdoorTrack, Gender::Boys, 2026)?,
            RegionAssertionDispositionV1::OtherSport,
        ),
        (
            context(Sport::CrossCountry, Gender::Girls, 2026)?,
            RegionAssertionDispositionV1::OtherCategory,
        ),
        (
            context(Sport::CrossCountry, Gender::Mixed, 2026)?,
            RegionAssertionDispositionV1::OtherCategory,
        ),
        (
            context(Sport::CrossCountry, Gender::Boys, 2025)?,
            RegionAssertionDispositionV1::OtherSchoolYear,
        ),
    ] {
        let payload = contract(vec![replace_assertion_context(&fact, context)?])?;
        let result = resolve_region_membership_v1(&payload, &query);
        check!(eq; result.verdict(), RegionMembershipVerdictV1::Unknown);
        check!(result.trace().all(|trace| trace.disposition == disposition));
    }
    Ok(())
}

#[test]
fn contradictory_explicit_evidence_survives_order_and_later_retrieval() -> TestResult {
    let school = school("a", UsJurisdiction::Michigan)?;
    let positive = assertion(
        RegionSubjectV1::School(school.clone()),
        RegionStatementV1::Member,
        "member",
    )?;
    let negative = assertion(
        RegionSubjectV1::School(school.clone()),
        RegionStatementV1::ExplicitNonMember,
        "excluded",
    )?;
    let mut negative_input = negative.input().clone();
    let mut provenance = negative_input.provenance.input().clone();
    provenance.evidence.observed_on = "2026-10-12T12:00:00Z".to_string();
    negative_input.provenance = RegionEvidenceV1::new(provenance)?;
    let negative = RegionAssertionV1::new(negative_input);
    let left = contract(vec![positive.clone(), negative.clone()])?;
    let right = contract(vec![negative, positive])?;
    check!(eq; left.reference()?, right.reference()?);
    let query = query(RegionMembershipTargetV1::School(school), "2026-10-01")?;
    let result = resolve_region_membership_v1(&left, &query);
    check!(eq; result.verdict(), RegionMembershipVerdictV1::Conflicted);
    check!(eq; result.trace().filter(|trace| trace.disposition == RegionAssertionDispositionV1::Applied).count(), 2);
    Ok(())
}

#[test]
fn unknown_fact_is_retained_beside_supported_member() -> TestResult {
    let school = school("a", UsJurisdiction::Michigan)?;
    let payload = contract(vec![
        assertion(
            RegionSubjectV1::School(school.clone()),
            RegionStatementV1::Member,
            "positive",
        )?,
        assertion(
            RegionSubjectV1::School(school.clone()),
            RegionStatementV1::Unknown,
            "unresolved",
        )?,
    ])?;
    let query = query(RegionMembershipTargetV1::School(school), "2026-10-01")?;
    let result = resolve_region_membership_v1(&payload, &query);
    check!(eq; result.verdict(), RegionMembershipVerdictV1::Member);
    check!(eq; result.trace().filter(|trace| trace.disposition == RegionAssertionDispositionV1::UnknownAssertion).count(), 1);
    Ok(())
}

#[test]
fn out_of_region_affiliation_does_not_inherit_meet_location_or_another_school() -> TestResult {
    let in_scope = school("a", UsJurisdiction::Michigan)?;
    let outside = school("b", UsJurisdiction::Wisconsin)?;
    let payload = contract(vec![assertion(
        RegionSubjectV1::School(in_scope.clone()),
        RegionStatementV1::Member,
        "positive",
    )?])?;
    let outside_query = query(RegionMembershipTargetV1::School(outside), "2026-10-01")?;
    check!(eq; resolve_region_membership_v1(&payload, &outside_query).verdict(), RegionMembershipVerdictV1::Unknown);
    let query = query(RegionMembershipTargetV1::School(in_scope), "2026-10-01")?;
    check!(eq; resolve_region_membership_v1(&payload, &query).verdict(), RegionMembershipVerdictV1::Member);
    Ok(())
}

#[test]
fn same_school_id_conflicting_jurisdiction_is_not_last_writer_wins() -> TestResult {
    let school = school("a", UsJurisdiction::Michigan)?;
    let foreign_claim = RegionSchoolV1::new(school.school().clone(), UsJurisdiction::Wisconsin)?;
    let payload = contract(vec![
        assertion(
            RegionSubjectV1::School(school.clone()),
            RegionStatementV1::Member,
            "positive",
        )?,
        assertion(
            RegionSubjectV1::School(foreign_claim),
            RegionStatementV1::Member,
            "foreign-location",
        )?,
    ])?;
    let query = query(RegionMembershipTargetV1::School(school), "2026-10-01")?;
    let result = resolve_region_membership_v1(&payload, &query);
    check!(eq; result.verdict(), RegionMembershipVerdictV1::Conflicted);
    check!(eq; result.trace().filter(|trace| trace.disposition == RegionAssertionDispositionV1::JurisdictionConflict).count(), 1);
    Ok(())
}

#[test]
fn contract_effective_interval_bounds_an_otherwise_applicable_assertion() -> TestResult {
    let school = school("contract-window", UsJurisdiction::Michigan)?;
    let fact = assertion(
        RegionSubjectV1::School(school.clone()),
        RegionStatementV1::Member,
        "wide-assertion-window",
    )?;
    let mut fact_input = fact.input().clone();
    fact_input.effective = RegionEffectiveV1::Inclusive(range("2025-01-01", "2028-12-31")?);
    let mut contract_input = contract(vec![RegionAssertionV1::new(fact_input)])?
        .input()
        .clone();
    contract_input.effective = range("2026-08-01", "2027-07-31")?;
    let payload = RegionMembershipContractV1::new(contract_input)?;
    for (effective_on, verdict, disposition) in [
        (
            "2026-07-31",
            RegionMembershipVerdictV1::Unknown,
            RegionAssertionDispositionV1::OutsideContractInterval,
        ),
        (
            "2026-08-01",
            RegionMembershipVerdictV1::Member,
            RegionAssertionDispositionV1::Applied,
        ),
        (
            "2027-07-31",
            RegionMembershipVerdictV1::Member,
            RegionAssertionDispositionV1::Applied,
        ),
        (
            "2027-08-01",
            RegionMembershipVerdictV1::Unknown,
            RegionAssertionDispositionV1::OutsideContractInterval,
        ),
    ] {
        let query = query(
            RegionMembershipTargetV1::School(school.clone()),
            effective_on,
        )?;
        let result = resolve_region_membership_v1(&payload, &query);
        check!(eq; result.verdict(), verdict);
        check!(eq; result.trace().len(), 1);
        let trace = result
            .trace()
            .next()
            .ok_or_else(|| std::io::Error::other("missing bounded assertion trace"))?;
        check!(eq; trace.disposition, disposition);
    }
    Ok(())
}
