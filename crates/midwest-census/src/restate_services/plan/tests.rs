//! The planner's dispositions: what a machine can sweep, what it must refuse by name, and the two
//! ways a plan goes wrong - dropping a source the research evidences, or inventing sources for a
//! jurisdiction the research says nothing about.

use census_crawl::registry::descriptor;
use census_domain::UsJurisdiction;

use super::*;
use crate::restate_services::jurisdiction::DISPATCHED;
use crate::restate_services::wire::{JurisdictionState, RefusedSource, SourcePlan};

/// A browser-session source with no lane behind it is refused, and the refusal names what is
/// missing: the run records it as owed work, so the reason has to reach the record.
#[test]
fn a_browser_source_without_a_lane_is_refused_by_name() {
    let disposition = classify_access(
        "athleticnet",
        AccessClass::BrowserSession,
        Dispatch::Wired,
        BrowserLaneState::Absent,
    );
    let refusal = match disposition {
        UnitDisposition::Refused(refusal) => refusal,
        other => panic!("a browser source with no lane must be refused, not {other:?}"),
    };
    assert_eq!(refusal.slug, "athleticnet");
    assert_eq!(refusal.access, AccessClass::BrowserSession);
    assert_eq!(refusal.reason, NO_BROWSER_LANE);
}

/// The same source on a machine that has the lane is ordinary sweeps, not a refusal.
#[test]
fn a_browser_source_with_a_lane_is_swept() {
    let disposition = classify_access(
        "athleticnet",
        AccessClass::BrowserSession,
        Dispatch::Wired,
        BrowserLaneState::Configured,
    );
    assert_eq!(
        disposition,
        UnitDisposition::Sweep(PlannedUnit {
            slug: "athleticnet",
            access: AccessClass::BrowserSession,
        })
    );
}

/// No lane is configured on the build machine, so an open source must never be gated on one.
#[test]
fn an_open_source_needs_no_lane() {
    let disposition = classify_access(
        "mshsl",
        AccessClass::Open,
        Dispatch::Wired,
        BrowserLaneState::Absent,
    );
    assert_eq!(
        disposition,
        UnitDisposition::Sweep(PlannedUnit {
            slug: "mshsl",
            access: AccessClass::Open,
        })
    );
}

/// A source no stage runs is owed by name, and the reason says the build is what is missing rather
/// than the machine: a lane, a setting or a rate would not make it runnable.
#[test]
fn a_source_no_stage_runs_is_owed_by_name() {
    let disposition = classify_access(
        "wiaa",
        AccessClass::Open,
        Dispatch::Unwired,
        BrowserLaneState::Configured,
    );
    let refusal = match disposition {
        UnitDisposition::Refused(refusal) => refusal,
        other => panic!("a source no stage runs must be refused, not {other:?}"),
    };
    assert_eq!(refusal.slug, "wiaa");
    assert_eq!(
        refusal.access,
        AccessClass::Open,
        "the refusal still records how it would have been acquired"
    );
    assert_eq!(refusal.reason, NO_JURISDICTION_WALK);
}

/// The dispatch question is asked before the lane one: with both gaps present the reason names the
/// one an operator cannot fix, because "start a lane and re-run" would be a promise this build
/// cannot keep for a source it has no stage for.
#[test]
fn the_dispatch_question_is_asked_before_the_lane() {
    let disposition = classify_access(
        "athleticnet",
        AccessClass::BrowserSession,
        Dispatch::Unwired,
        BrowserLaneState::Absent,
    );
    let refusal = match disposition {
        UnitDisposition::Refused(refusal) => refusal,
        other => panic!("a source with both gaps must be refused, not {other:?}"),
    };
    assert_eq!(refusal.reason, NO_JURISDICTION_WALK);
}

/// The plan's central claim, held for every jurisdiction: nothing is sweepable that the chain does
/// not dispatch, so a unit in the sweepable list is work some stage will actually run.
#[test]
fn nothing_sweepable_is_a_source_no_stage_runs() {
    for jurisdiction in [
        UsJurisdiction::Wisconsin,
        UsJurisdiction::Minnesota,
        UsJurisdiction::Illinois,
        UsJurisdiction::Ohio,
    ] {
        let dispositions = plan(jurisdiction, BrowserLaneState::Configured);
        for unit in sweepable(&dispositions) {
            assert!(
                DISPATCHED.contains(&unit.slug),
                "{jurisdiction}: {unit:?} is planned as sweepable but no stage sweeps it"
            );
        }
    }
}

/// The state association's directory walk is planned where it covers and nowhere else: the plan is
/// what a run dispatches, and a directory swept for another state's schools would write rows the
/// walk never fetched. The applicability table is what decides, so this test is where its answer
/// for the two states the walks cover is written down.
#[test]
fn a_states_own_directory_walk_is_planned_only_for_that_state() {
    let slugs = |jurisdiction: UsJurisdiction| -> Vec<&'static str> {
        sweepable(&plan(jurisdiction, BrowserLaneState::Configured))
            .iter()
            .map(|unit| unit.slug)
            .collect()
    };

    let wisconsin = slugs(UsJurisdiction::Wisconsin);
    assert!(wisconsin.contains(&"wiaa"), "{wisconsin:?}");
    assert!(
        !wisconsin.contains(&"mshsl"),
        "Minnesota's directory is not Wisconsin's work: {wisconsin:?}"
    );

    let minnesota = slugs(UsJurisdiction::Minnesota);
    assert!(minnesota.contains(&"mshsl"), "{minnesota:?}");
    assert!(
        !minnesota.contains(&"wiaa"),
        "Wisconsin's directory is not Minnesota's work: {minnesota:?}"
    );

    // One adapter covers two states, and each run walks its own half: the plan is the only thing
    // that decides which, so both are held here.
    for (jurisdiction, neighbour) in [
        (UsJurisdiction::Nebraska, UsJurisdiction::NorthDakota),
        (UsJurisdiction::NorthDakota, UsJurisdiction::Nebraska),
    ] {
        let planned = slugs(jurisdiction);
        assert!(
            planned.contains(&"plain_names"),
            "{jurisdiction}: {planned:?}"
        );
        assert!(
            !planned.contains(&"wiaa") && !planned.contains(&"mshsl"),
            "{jurisdiction} is not the Wisconsin or Minnesota directory's work: {planned:?}"
        );
        let other = slugs(neighbour);
        assert!(other.contains(&"plain_names"), "{neighbour}: {other:?}");
    }
}

/// The meet-index sources are planned where their own index covers and nowhere else: the result
/// archive is one association's publication and the timer publishes its own states' schedules, so a
/// run of another state that planned them would walk meets it was never asked for.
#[test]
fn the_meet_walks_are_planned_for_the_states_they_publish() {
    let slugs = |jurisdiction: UsJurisdiction| -> Vec<&'static str> {
        sweepable(&plan(jurisdiction, BrowserLaneState::Configured))
            .iter()
            .map(|unit| unit.slug)
            .collect()
    };

    let wisconsin = slugs(UsJurisdiction::Wisconsin);
    assert!(wisconsin.contains(&"wiaa_results"), "{wisconsin:?}");

    for jurisdiction in [UsJurisdiction::Minnesota, UsJurisdiction::Iowa] {
        let planned = slugs(jurisdiction);
        assert!(planned.contains(&"wayzata"), "{jurisdiction}: {planned:?}");
    }

    for jurisdiction in [UsJurisdiction::Nebraska, UsJurisdiction::Ohio] {
        let planned = slugs(jurisdiction);
        assert!(
            !planned.contains(&"wiaa_results") && !planned.contains(&"wayzata"),
            "{jurisdiction} is not the archive's or the timer's work: {planned:?}"
        );
    }
}

/// The dispatched slugs are the registry's, not a second spelling of them: a rename that left this
/// list behind would refuse every source and report a machine gap that is really a typo.
#[test]
fn every_dispatched_slug_is_registered() {
    for slug in DISPATCHED {
        assert!(
            descriptor(slug).is_some(),
            "{slug} is dispatched by the chain but names no registered source"
        );
    }
}

/// The two partitions a dispatcher reads: refusals never appear in the fetched set, and both keep
/// the plan's order.
#[test]
fn owed_keeps_plan_order_and_sweepable_excludes_refusals() {
    let dispositions = vec![
        classify_access(
            "tfrrs",
            AccessClass::BrowserSession,
            Dispatch::Wired,
            BrowserLaneState::Absent,
        ),
        classify_access(
            "mshsl",
            AccessClass::Open,
            Dispatch::Wired,
            BrowserLaneState::Absent,
        ),
        classify_access(
            "wiha",
            AccessClass::BrowserSession,
            Dispatch::Wired,
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
            Dispatch::Wired,
            BrowserLaneState::Absent,
        ),
        classify_access(
            "mshsl",
            AccessClass::Open,
            Dispatch::Wired,
            BrowserLaneState::Absent,
        ),
        classify_access(
            "wiha",
            AccessClass::BrowserSession,
            Dispatch::Wired,
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
            Dispatch::Wired,
            BrowserLaneState::Absent,
        ),
        classify_access(
            "mshsl",
            AccessClass::Open,
            Dispatch::Wired,
            BrowserLaneState::Absent,
        ),
    ];
    state.plan = Some(SourcePlan::of(&dispositions));
    let written = serde_json::to_value(&state).expect("a state serializes");
    let read: JurisdictionState = serde_json::from_value(written).expect("and reads back");
    assert_eq!(read.plan, state.plan);
    let plan = read.plan.expect("the state carried a plan");
    assert_eq!(plan.sweepable, vec!["mshsl".to_string()]);
    assert_eq!(plan.refused.len(), 1);
}
