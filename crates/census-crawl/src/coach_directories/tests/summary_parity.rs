use super::*;
use census_domain::model::{CoachRole, Gender, SchoolId, Sport};
use std::collections::BTreeSet;

#[test]
fn captured_summary_keeps_four_varsity_contexts_and_counts_four_jv_rejections() -> TestResult {
    let summary = parse_summary(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/coach_directories/probe/WY/summary-SS28UB.json"
    )))?;
    let emission = coach_entities(
        &summary,
        &SchoolId::mint("sch", &["arapaho-charter"]),
        "https://example.test/schools/SS28UB/summary",
        "2026-09-30",
        EmissionScope::Census,
    )?;
    let contexts: BTreeSet<_> = emission
        .coaches
        .iter()
        .map(|coach| (coach.name.as_str(), coach.sport, coach.gender, coach.role))
        .collect();
    check!(eq;
        contexts,
        BTreeSet::from([
            (
                "Nicole Biltoft",
                Some(Sport::CrossCountry),
                Gender::Boys,
                CoachRole::Unknown
            ),
            (
                "Nicole Biltoft",
                Some(Sport::CrossCountry),
                Gender::Girls,
                CoachRole::Unknown
            ),
            (
                "Nicole Biltoft",
                Some(Sport::OutdoorTrack),
                Gender::Boys,
                CoachRole::Unknown
            ),
            (
                "Nicole Biltoft",
                Some(Sport::OutdoorTrack),
                Gender::Girls,
                CoachRole::Unknown
            ),
        ])
    );
    check!(eq; emission.coaches.len(), contexts.len());
    check!(eq;
        emission.counters.dropped_levels,
        std::collections::BTreeMap::from([("JV".to_string(), 4)])
    );
    check!(eq; emission.counters.dropped_person, 0);
    check!(eq; emission.counters.dropped_vendor, 0);
    Ok(())
}
