use super::*;

fn program_assertion(
    composition: RegionCompositionV1,
    label: &str,
) -> TestResult<RegionAssertionV1> {
    let subject = RegionSubjectV1::Program(RegionProgramV1::new(program("coop"), composition)?);
    assertion(subject, RegionStatementV1::Member, label)
}

#[test]
fn co_op_membership_requires_direct_program_and_affiliated_constituent() -> TestResult {
    let a = school("a", UsJurisdiction::Michigan)?;
    let b = school("b", UsJurisdiction::Michigan)?;
    let c = school("c", UsJurisdiction::Michigan)?;
    let composition = RegionCompositionV1::co_op(vec![b.clone(), a.clone()])?;
    let payload = contract(vec![program_assertion(composition, "co-op-assignment")?])?;
    for member in [a.clone(), b] {
        let query = query(
            RegionMembershipTargetV1::Program {
                program: program("coop"),
                affiliated_school: member,
            },
            "2026-10-01",
        )?;
        check!(eq; resolve_region_membership_v1(&payload, &query).verdict(), RegionMembershipVerdictV1::Member);
    }
    let school_query = query(RegionMembershipTargetV1::School(a), "2026-10-01")?;
    check!(eq; resolve_region_membership_v1(&payload, &school_query).verdict(), RegionMembershipVerdictV1::Unknown);
    let foreign_query = query(
        RegionMembershipTargetV1::Program {
            program: program("coop"),
            affiliated_school: c,
        },
        "2026-10-01",
    )?;
    let result = resolve_region_membership_v1(&payload, &foreign_query);
    check!(eq; result.verdict(), RegionMembershipVerdictV1::Unknown);
    check!(result
        .trace()
        .all(|trace| trace.disposition == RegionAssertionDispositionV1::ForeignConstituent));
    Ok(())
}

#[test]
fn school_membership_does_not_assign_membership_to_unbound_program() -> TestResult {
    let a = school("a", UsJurisdiction::Michigan)?;
    let payload = contract(vec![assertion(
        RegionSubjectV1::School(a.clone()),
        RegionStatementV1::Member,
        "school",
    )?])?;
    let query = query(
        RegionMembershipTargetV1::Program {
            program: program("unbound"),
            affiliated_school: a,
        },
        "2026-10-01",
    )?;
    check!(eq; resolve_region_membership_v1(&payload, &query).verdict(), RegionMembershipVerdictV1::Unknown);
    Ok(())
}

#[test]
fn unresolved_constituents_are_retained_without_establishing_membership() -> TestResult {
    let payload = contract(vec![program_assertion(
        RegionCompositionV1::unknown(),
        "unknown-constituents",
    )?])?;
    let a = school("a", UsJurisdiction::Michigan)?;
    let query = query(
        RegionMembershipTargetV1::Program {
            program: program("coop"),
            affiliated_school: a,
        },
        "2026-10-01",
    )?;
    let result = resolve_region_membership_v1(&payload, &query);
    check!(eq; result.verdict(), RegionMembershipVerdictV1::Unknown);
    check!(result
        .trace()
        .all(|trace| trace.disposition == RegionAssertionDispositionV1::UnresolvedConstituents));
    Ok(())
}

#[test]
fn conflicting_constituent_sets_survive_even_when_only_one_contains_affiliation() -> TestResult {
    let a = school("a", UsJurisdiction::Michigan)?;
    let b = school("b", UsJurisdiction::Michigan)?;
    let c = school("c", UsJurisdiction::Michigan)?;
    let payload = contract(vec![
        program_assertion(
            RegionCompositionV1::co_op(vec![a.clone(), b.clone()])?,
            "co-op-ab",
        )?,
        program_assertion(RegionCompositionV1::co_op(vec![b, c])?, "co-op-bc")?,
    ])?;
    let query = query(
        RegionMembershipTargetV1::Program {
            program: program("coop"),
            affiliated_school: a,
        },
        "2026-10-01",
    )?;
    let result = resolve_region_membership_v1(&payload, &query);
    check!(eq; result.verdict(), RegionMembershipVerdictV1::Conflicted);
    check!(eq; result.composition(), RegionCompositionResolutionV1::Conflicted);
    check!(eq; result.trace().len(), 2);
    Ok(())
}

#[test]
fn dated_co_op_constituent_changes_do_not_conflict_across_periods() -> TestResult {
    let a = school("a", UsJurisdiction::Michigan)?;
    let b = school("b", UsJurisdiction::Michigan)?;
    let c = school("c", UsJurisdiction::Michigan)?;
    let old = program_assertion(RegionCompositionV1::co_op(vec![a.clone(), b])?, "old-ab")?;
    let mut old_input = old.input().clone();
    old_input.effective = RegionEffectiveV1::Inclusive(range("2026-08-01", "2026-09-30")?);
    let new = program_assertion(RegionCompositionV1::co_op(vec![a.clone(), c])?, "new-ac")?;
    let mut new_input = new.input().clone();
    new_input.effective = RegionEffectiveV1::Inclusive(range("2026-10-01", "2027-07-31")?);
    let payload = contract(vec![
        RegionAssertionV1::new(old_input),
        RegionAssertionV1::new(new_input),
    ])?;
    for effective_on in ["2026-09-30", "2026-10-01"] {
        let query = query(
            RegionMembershipTargetV1::Program {
                program: program("coop"),
                affiliated_school: a.clone(),
            },
            effective_on,
        )?;
        let result = resolve_region_membership_v1(&payload, &query);
        check!(eq; result.verdict(), RegionMembershipVerdictV1::Member);
        check!(eq; result.trace().filter(|trace| trace.disposition == RegionAssertionDispositionV1::OutsideAssertionInterval).count(), 1);
    }
    Ok(())
}

#[test]
fn co_op_raw_limits_duplicates_and_jurisdiction_conflicts_are_checked() -> TestResult {
    let a = school("a", UsJurisdiction::Michigan)?;
    let b = school("b", UsJurisdiction::Michigan)?;
    check!(RegionCompositionV1::co_op(vec![a.clone()]).is_err());
    check!(RegionCompositionV1::co_op(vec![a.clone(), a.clone()]).is_err());
    let foreign = RegionSchoolV1::new(a.school().clone(), UsJurisdiction::Wisconsin)?;
    check!(RegionCompositionV1::co_op(vec![a.clone(), foreign]).is_err());
    let mut at_cap = vec![a.clone(); MAX_REGION_CONSTITUENTS];
    if let Some(last) = at_cap.last_mut() {
        *last = b.clone();
    } else {
        return Err(std::io::Error::other("missing bounded fixture slot").into());
    }
    check!(eq; RegionCompositionV1::co_op(at_cap)?.schools().len(), 2);
    let over_cap = vec![
        a;
        MAX_REGION_CONSTITUENTS
            .checked_add(1)
            .ok_or_else(|| std::io::Error::other("fixture overflow"))?
    ];
    check!(matches!(
        RegionCompositionV1::co_op(over_cap),
        Err(RegionMembershipError::TooManyEntries {
            field: "constituents",
            ..
        })
    ));
    let malformed = serde_json::json!({ "co_op": [serde_json::to_value(b)?] });
    check!(serde_json::from_value::<RegionCompositionV1>(malformed).is_err());
    Ok(())
}
