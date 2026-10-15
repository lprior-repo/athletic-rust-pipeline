use super::*;
use census_domain::model::{ExactSeconds, Id, Mark, PerformanceIdentity, PerformanceResult};
use std::collections::BTreeSet;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn performance(label: &str) -> Result<CanonicalPerformance, Box<dyn std::error::Error>> {
    let athlete = Id::mint("ath", &["result-group-fixture"]);
    let meet = Id::mint("meet", &["result-group-fixture"]);
    let event = Id::mint("event", &["result-group-fixture"]);
    let team = Id::mint("team", &["result-group-fixture"]);
    Ok(CanonicalPerformance::new(
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
    )?)
}

#[test]
fn historical_policies_keep_equal_missing_context_for_distinct_results() -> TestResult {
    let first = performance("result-one")?;
    let second = performance("result-two")?;
    check!(first.id != second.id);
    for policy in [
        EventSemantics::Legacy18,
        EventSemantics::FlatEvents19,
        EventSemantics::CombinedEvents20,
    ] {
        check!(ReportGroupKey::of(&first, policy) == ReportGroupKey::of(&second, policy));
    }
    Ok(())
}

#[test]
fn result_group_policy_distinguishes_results_with_absent_round_and_heat() -> TestResult {
    let first = performance("result-one")?;
    let second = performance("result-two")?;
    check!(first.round.is_none() && first.heat.is_none());
    check!(second.round.is_none() && second.heat.is_none());
    let policy = EventSemantics::ResultGroups21;
    check!(ReportGroupKey::of(&first, policy) != ReportGroupKey::of(&second, policy));
    Ok(())
}

#[test]
fn result_group_policy_deduplicates_exact_id_replay() -> TestResult {
    let first = performance("result-one")?;
    let replay = first.clone();
    let policy = EventSemantics::ResultGroups21;
    let keys = BTreeSet::from([
        ReportGroupKey::of(&first, policy),
        ReportGroupKey::of(&replay, policy),
    ]);
    check!(eq; keys.len(), 1);
    Ok(())
}

#[test]
fn source_key_text_cannot_coalesce_distinct_performance_ids() -> TestResult {
    let first = performance("milesplit_result:201782263")?;
    let mut second = performance("another-retained-identity")?;
    second.source_key = first.source_key.clone();
    second.evidence = first.evidence.clone();
    let policy = EventSemantics::ResultGroups21;
    check!(ReportGroupKey::of(&first, policy) != ReportGroupKey::of(&second, policy));
    Ok(())
}

#[test]
fn group_identity_does_not_depend_on_result_value() -> TestResult {
    let first = performance("result-one")?;
    let mut correction = first.clone();
    correction.mark = Mark::TimeSeconds(ExactSeconds::parse("51.00")?);
    for policy in [
        EventSemantics::CombinedEvents20,
        EventSemantics::ResultGroups21,
    ] {
        check!(ReportGroupKey::of(&first, policy) == ReportGroupKey::of(&correction, policy));
    }
    Ok(())
}

#[test]
fn historical_context_fields_stay_material_without_splitting_exact_id_corrections() -> TestResult {
    let first = performance("result-one")?;
    let mut variants = vec![first.clone(); 5];
    variants.get_mut(0).ok_or("meet variant")?.meet = Id::mint("meet", &["other"]);
    variants.get_mut(1).ok_or("date variant")?.date = "2026-04-02".into();
    variants.get_mut(2).ok_or("event variant")?.event = Id::mint("event", &["other"]);
    variants.get_mut(3).ok_or("round variant")?.round = Some("Final".into());
    variants.get_mut(4).ok_or("heat variant")?.heat = Some("1".into());
    for changed in &variants {
        check!(
            ReportGroupKey::of(&first, EventSemantics::CombinedEvents20)
                != ReportGroupKey::of(changed, EventSemantics::CombinedEvents20)
        );
        check!(
            ReportGroupKey::of(&first, EventSemantics::ResultGroups21)
                == ReportGroupKey::of(changed, EventSemantics::ResultGroups21)
        );
    }
    Ok(())
}

#[test]
fn conflict_membership_isolated_to_exact_result_group() -> TestResult {
    let conflict = performance("conflicted-result")?;
    let clean = performance("independent-result")?;
    let replay = conflict.clone();
    let policy = EventSemantics::ResultGroups21;
    let blocked = BTreeSet::from([ReportGroupKey::of(&conflict, policy)]);
    check!(blocked.contains(&ReportGroupKey::of(&replay, policy)));
    check!(!blocked.contains(&ReportGroupKey::of(&clean, policy)));
    Ok(())
}

#[test]
fn historical_conflict_membership_still_covers_the_whole_context() -> TestResult {
    let conflict = performance("conflicted-result")?;
    let clean = performance("independent-result")?;
    for policy in [
        EventSemantics::Legacy18,
        EventSemantics::FlatEvents19,
        EventSemantics::CombinedEvents20,
    ] {
        let blocked = BTreeSet::from([ReportGroupKey::of(&conflict, policy)]);
        check!(blocked.contains(&ReportGroupKey::of(&clean, policy)));
    }
    Ok(())
}

#[test]
fn blocked_terminal_result_cannot_poison_an_independent_result_group() -> TestResult {
    let mut terminal = performance("terminal-result")?;
    terminal.mark = Mark::Raw("DNF".into());
    let clean = performance("complete-result")?;
    let policy = EventSemantics::ResultGroups21;
    let blocked = BTreeSet::from([ReportGroupKey::of(&terminal, policy)]);
    check!(!blocked.contains(&ReportGroupKey::of(&clean, policy)));
    Ok(())
}
