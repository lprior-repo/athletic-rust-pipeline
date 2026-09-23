//! The planner's dispositions: what a machine can sweep, what it must refuse by name, and the two
//! ways a plan goes wrong - dropping a source the research evidences, or inventing sources for a
//! jurisdiction the research says nothing about.

use super::*;
use census_domain::UsJurisdiction;

/// A browser-session source with no lane behind it is refused, and the refusal names what is
/// missing: the run records it as owed work, so the reason has to reach the record.
#[test]
fn a_browser_source_without_a_lane_is_refused_by_name() {
    let disposition = classify_access("tfrrs", AccessClass::BrowserSession, BrowserLaneState::Absent);
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
    let disposition = classify_access("tfrrs", AccessClass::BrowserSession, BrowserLaneState::Configured);
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
        classify_access("tfrrs", AccessClass::BrowserSession, BrowserLaneState::Absent),
        classify_access("mshsl", AccessClass::Open, BrowserLaneState::Absent),
        classify_access("wiha", AccessClass::BrowserSession, BrowserLaneState::Absent),
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
