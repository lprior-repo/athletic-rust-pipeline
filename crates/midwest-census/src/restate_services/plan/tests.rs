//! The planner's dispositions: what a machine can sweep, what it must refuse by name, and the two
//! ways a plan goes wrong - dropping a source the research evidences, or inventing sources for a
//! jurisdiction the research says nothing about.

use super::*;
use crate::restate_services::wire::{JurisdictionState, RefusedSource, SourcePlan};
use census_domain::UsJurisdiction;

/// A browser-session source with no lane behind it is refused, and the refusal names what is
/// missing: the run records it as owed work, so the reason has to reach the record.
#[test]
fn a_browser_source_without_a_lane_is_refused_by_name() {
    let disposition = classify_access(
        "tfrrs",
        AccessClass::BrowserSession,
        BrowserLaneState::Absent,
    );
    let refusal = match disposition {
        UnitDisposition::Refused(refusal) => refusal,
        other => panic!("a browser source with no lane must be refused, not {other:?}"),
    };
    assert_eq!(refusal.slug, "tfrrs");
    assert_eq!(refusal.access, AccessClass::BrowserSession);
    assert_eq!(refusal.reason, NO_BROWSER_LANE);
}

/// The same source on a machine that has the lane is ordinary sweeps, not a refusal.
#[test]
fn a_browser_source_with_a_lane_is_swept() {
    let disposition = classify_access(
        "tfrrs",
        AccessClass::BrowserSession,
        BrowserLaneState::Configured,
    );
    assert_eq!(
        disposition,
        UnitDisposition::Sweep(PlannedUnit {
            slug: "tfrrs",
            access: AccessClass::BrowserSession,
        })
    );
}

/// No lane is configured on the build machine, so an open source must never be gated on one.
#[test]
fn an_open_source_needs_no_lane() {
    let disposition = classify_access("mshsl", AccessClass::Open, BrowserLaneState::Absent);
    assert_eq!(
        disposition,
        UnitDisposition::Sweep(PlannedUnit {
            slug: "mshsl",
            access: AccessClass::Open,
        })
    );
}

/// The two partitions a dispatcher reads: refusals never appear in the fetched set, and both keep
/// the plan's order.
#[test]
fn owed_keeps_plan_order_and_sweepable_excludes_refusals() {
    let dispositions = vec![
        classify_access(
            "tfrrs",
            AccessClass::BrowserSession,
            BrowserLaneState::Absent,
        ),
        classify_access("mshsl", AccessClass::Open, BrowserLaneState::Absent),
        classify_access(
            "wiha",
            AccessClass::BrowserSession,
            BrowserLaneState::Absent,
        ),
    ];
    let owed_slugs: Vec<&str> = owed(&dispositions)
        .iter()
        .map(|refusal| refusal.slug)
        .collect();
    let sweepable_slugs: Vec<&str> = sweepable(&dispositions)
        .iter()
        .map(|unit| unit.slug)
        .collect();
    assert_eq!(owed_slugs, vec!["tfrrs", "wiha"]);
    assert_eq!(sweepable_slugs, vec!["mshsl"]);
}

/// The plan is the applicability table's answer, not a second opinion: the same sources in the same
/// order, with none dropped between the two.
#[test]
fn a_jurisdiction_plans_the_sources_the_table_evidences_in_order() {
    for jurisdiction in [
        UsJurisdiction::Wisconsin,
        UsJurisdiction::Minnesota,
        UsJurisdiction::Illinois,
    ] {
        let expected: Vec<&str> = applicable_sources(jurisdiction)
            .iter()
            .map(|descriptor| descriptor.slug)
            .collect();
        let planned: Vec<&str> = plan(jurisdiction, BrowserLaneState::Absent)
            .iter()
            .map(UnitDisposition::slug)
            .collect();
        assert_eq!(planned, expected);
    }
}

/// Out of scope is not a licence to plan everything: a jurisdiction the research does not evidence
/// plans nothing, and an empty plan is a real answer.
#[test]
fn a_jurisdiction_the_research_does_not_evidence_plans_nothing() {
    assert!(plan(UsJurisdiction::Alaska, BrowserLaneState::Absent).is_empty());
}

/// The plan as a run records it: sweepable slugs in plan order, every refusal with the reason it is
/// owed, and no refused source left in the sweepable list.
#[test]
fn a_recorded_plan_partitions_the_dispositions_in_plan_order() {
    let dispositions = vec![
        classify_access(
            "tfrrs",
            AccessClass::BrowserSession,
            BrowserLaneState::Absent,
        ),
        classify_access("mshsl", AccessClass::Open, BrowserLaneState::Absent),
        classify_access(
            "wiha",
            AccessClass::BrowserSession,
            BrowserLaneState::Absent,
        ),
    ];
    let recorded = SourcePlan::of(&dispositions);
    assert_eq!(recorded.sweepable, vec!["mshsl".to_string()]);
    assert_eq!(
        recorded.refused,
        vec![
            RefusedSource {
                slug: "tfrrs".to_string(),
                reason: NO_BROWSER_LANE.to_string(),
            },
            RefusedSource {
                slug: "wiha".to_string(),
                reason: NO_BROWSER_LANE.to_string(),
            },
        ]
    );
}

/// A state journaled before the plan existed reads as absent rather than failing: that absence is
/// what makes the object build a plan on the next run instead of reporting a machine's gap as a
/// broken state.
#[test]
fn a_state_journaled_before_the_plan_reads_with_no_plan() {
    let state: JurisdictionState = serde_json::from_value(serde_json::json!({
        "identity": "jurisdiction:WI:2026-27:1"
    }))
    .expect("a state without a plan is still a state");
    assert!(state.plan.is_none());
}

/// What a run wrote is what a re-invocation reads back: the recorded plan survives the journal
/// whole, refusals and reasons included.
#[test]
fn a_recorded_plan_round_trips_through_the_journal() {
    let mut state = JurisdictionState::default();
    let dispositions = vec![
        classify_access(
            "tfrrs",
            AccessClass::BrowserSession,
            BrowserLaneState::Absent,
        ),
        classify_access("mshsl", AccessClass::Open, BrowserLaneState::Absent),
    ];
    state.plan = Some(SourcePlan::of(&dispositions));
    let written = serde_json::to_value(&state).expect("a state serializes");
    let read: JurisdictionState = serde_json::from_value(written).expect("and reads back");
    assert_eq!(read.plan, state.plan);
    let plan = read.plan.expect("the state carried a plan");
    assert_eq!(plan.sweepable, vec!["mshsl".to_string()]);
    assert_eq!(plan.refused.len(), 1);
}
