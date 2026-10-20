use super::*;
use census_domain::model::{
    Adjustment, AdjustmentBasis, Evidence, ExactSeconds, Grade, Id, Mark, PerformanceIdentity,
    PerformanceResult, SourceIdentity, SourceNamespace, SourceRef, TimingMethod,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn performance(label: &str) -> Result<CanonicalPerformance, Box<dyn std::error::Error>> {
    let athlete = Id::mint("ath", &["identity-admission"]);
    let meet = Id::mint("meet", &["identity-admission"]);
    let event = Id::mint("event", &["identity-admission"]);
    let team = Id::mint("team", &["identity-admission"]);
    let mut row = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &athlete,
            meet: &meet,
            event: &event,
            date: "2026-04-01",
            source_key: label,
        },
        PerformanceResult {
            team: &team,
            mark: Mark::TimeSeconds(ExactSeconds::parse("50.00")?),
            adjustment: None,
            wind_mps: None,
            place: None,
        },
    )?;
    row.source_athlete = Some(SourceIdentity::new(
        SourceNamespace::MilesplitAthlete,
        "runner",
    ));
    Ok(row)
}

fn refuses(first: &CanonicalPerformance, second: &CanonicalPerformance) -> TestResult {
    for rows in [
        [first.clone(), second.clone()],
        [second.clone(), first.clone()],
    ] {
        let error = admit(&rows, EventSemantics::ResultGroups21)
            .err()
            .ok_or("accepted contradiction")?;
        check!(eq; error.to_string(), format!("performance {} has incompatible retained identity or context", first.id));
    }
    Ok(())
}

fn changed(
    row: &CanonicalPerformance,
    change: impl FnOnce(&mut CanonicalPerformance),
) -> CanonicalPerformance {
    let mut changed = row.clone();
    change(&mut changed);
    changed
}

#[test]
fn repeated_id_requires_each_identity_and_context_field_to_agree() -> TestResult {
    let first = performance("one")?;
    let cases = [
        changed(&first, |r| r.athlete = Id::mint("ath", &["other"])),
        changed(&first, |r| r.team = Id::mint("team", &["other"])),
        changed(&first, |r| r.event = Id::mint("event", &["other"])),
        changed(&first, |r| r.meet = Id::mint("meet", &["other"])),
        changed(&first, |r| r.date = "2026-04-02".into()),
        changed(&first, |r| r.source_key = "other".into()),
        changed(&first, |r| r.heat = Some("2".into())),
        changed(&first, |r| r.round = Some("final".into())),
        changed(&first, |r| r.timing = Some(TimingMethod::Hand)),
        changed(&first, |r| r.wind_mps = Some(2.5)),
        changed(&first, |r| r.place = Some(2)),
        changed(&first, |r| r.observed_grade = Grade::new(11)),
    ];
    cases.iter().try_for_each(|other| refuses(&first, other))
}

#[test]
fn repeated_id_requires_present_source_namespace_and_id_to_agree() -> TestResult {
    let first = performance("one")?;
    for source in [
        None,
        Some(SourceIdentity::new(
            SourceNamespace::MilesplitAthlete,
            "other",
        )),
        Some(SourceIdentity::new(
            SourceNamespace::Other("timer".into()),
            "runner",
        )),
    ] {
        let mut other = first.clone();
        other.source_athlete = source;
        refuses(&first, &other)?;
    }
    Ok(())
}

#[test]
fn repeated_id_requires_adjustment_presence_and_comparison_basis_to_agree() -> TestResult {
    let first = performance("one")?;
    let mut adjusted = first.clone();
    adjusted.adjustment = Some(Adjustment::track_size_conversion(
        Some(first.mark.clone()),
        first.mark.clone(),
        None,
        None,
    ));
    refuses(&first, &adjusted)?;
    let mut unknown = adjusted.clone();
    unknown.adjustment.as_mut().ok_or("adjustment")?.basis = AdjustmentBasis::Unknown;
    refuses(&adjusted, &unknown)
}

#[test]
fn evidence_and_source_url_variation_do_not_change_result_ownership() -> TestResult {
    let first = performance("one")?;
    let mut other = first.clone();
    other.source_athlete.as_mut().ok_or("owner")?.url = Some("https://example.test/new-url".into());
    other.evidence = vec![Evidence::parsed(
        SourceRef::new("timer", None),
        "2026-04-03",
    )];
    for rows in [[first.clone(), other.clone()], [other, first]] {
        admit(&rows, EventSemantics::ResultGroups21)?;
    }
    Ok(())
}

#[test]
fn historical_policies_do_not_acquire_current_identity_admission() -> TestResult {
    let first = performance("one")?;
    let other = changed(&first, |row| row.athlete = Id::mint("ath", &["other"]));
    for policy in [
        EventSemantics::FlatEvents19,
        EventSemantics::CombinedEvents20,
    ] {
        admit(&[first.clone(), other.clone()], policy)?;
    }
    Ok(())
}

#[test]
fn distinct_ids_with_different_contexts_remain_admissible() -> TestResult {
    let first = performance("one")?;
    let other = changed(&performance("two")?, |row| {
        row.athlete = Id::mint("ath", &["other"])
    });
    admit(&[first, other], EventSemantics::ResultGroups21)?;
    Ok(())
}

#[test]
fn identity_admission_charges_before_new_index_allocation() -> TestResult {
    let first = performance("one")?;
    let second = performance("two")?;
    let mut index = Identities::new();
    let encoded = serde_json::to_vec(&first)?.len();
    let next_encoded = serde_json::to_vec(&second)?.len();
    index.budget.charge(
        crate::grouping::STAGE_BYTES - reference_bytes(&first.id)? - encoded - next_encoded,
        0,
    )?;
    index.record(&first)?;
    check!(index.record(&second).is_err());
    check!(eq; index.rows.len(), 1);
    check!(!index.rows.contains_key(&second.id));
    Ok(())
}

#[test]
fn identity_admission_charges_every_replayed_row() -> TestResult {
    let first = performance("one")?;
    let mut index = Identities::new();
    index.budget.charge(0, crate::grouping::STAGE_WORK - 1)?;
    index.record(&first)?;
    check!(index.record(&first).is_err());
    check!(eq; index.rows.len(), 1);
    Ok(())
}

#[test]
fn identity_admission_bounds_repeated_context_before_comparison() -> TestResult {
    let first = performance("one")?;
    let other = changed(&first, |row| {
        row.source_key = "x".repeat(crate::grouping::ROW_BYTES)
    });
    let mut index = Identities::new();
    index.record(&first)?;
    let error = index
        .record(&other)
        .err()
        .ok_or("oversized context accepted")?;
    check!(error.to_string().contains("encoded row bytes"));
    check!(eq; index.rows.len(), 1);
    Ok(())
}

#[test]
fn identity_admission_bounds_complete_provenance_at_the_exact_row_limit() -> TestResult {
    let mut row = performance("provenance-limit")?;
    row.evidence = vec![Evidence::derived(SourceRef::id("timer"), "2026-04-01", "")];
    let encoded = serde_json::to_vec(&row)?.len();
    let remaining = crate::grouping::ROW_BYTES
        .checked_sub(encoded)
        .ok_or("row budget")?;
    row.evidence.first_mut().ok_or("evidence")?.note = Some("x".repeat(remaining));
    check!(eq; serde_json::to_vec(&row)?.len(), crate::grouping::ROW_BYTES);
    admit(std::slice::from_ref(&row), EventSemantics::ResultGroups21)?;
    row.evidence
        .first_mut()
        .ok_or("evidence")?
        .note
        .as_mut()
        .ok_or("note")?
        .push('x');
    let error = admit(std::slice::from_ref(&row), EventSemantics::ResultGroups21)
        .err()
        .ok_or("oversized provenance accepted")?;
    check!(error.to_string().contains("encoded row bytes"));
    for policy in [
        EventSemantics::FlatEvents19,
        EventSemantics::CombinedEvents20,
    ] {
        admit(std::slice::from_ref(&row), policy)?;
    }
    Ok(())
}
