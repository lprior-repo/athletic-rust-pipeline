//! What the §31 join and the merge record say: which row a source object keyed, which id a decision
//! retired.

use super::super::*;
use crate::model::SourceNamespace;

#[test]
fn every_kind_has_a_distinct_table_slug() {
    let mut slugs: Vec<&str> = SourceEntityKind::ALL
        .iter()
        .map(|kind| kind.slug())
        .collect();
    slugs.sort_unstable();
    let count = slugs.len();
    slugs.dedup();
    assert_eq!(slugs.len(), count, "two kinds claim one table");
    for kind in SourceEntityKind::ALL {
        assert!(
            !kind.slug().is_empty() && kind.slug().chars().all(|c| c.is_ascii_lowercase()),
            "{kind:?} slug is not a table name"
        );
    }
}

#[test]
fn a_source_object_keys_on_the_source_not_the_canonical_row() {
    let first = SourceObjectIdentity::new(
        SourceNamespace::MilesplitAthlete,
        SourceEntityKind::Athletes,
        "12345",
        "ath:wi-abbotsford:jane-doe:2027:f",
    );
    let second = SourceObjectIdentity::new(
        SourceNamespace::MilesplitAthlete,
        SourceEntityKind::Athletes,
        "12345",
        "ath:wi-elsewhere:jane-doe:2027:f",
    );
    assert_eq!(
        first.id, second.id,
        "one source object is one row, whatever canonical row it joined"
    );
    assert_eq!(first.id, "milesplit_athlete:athletes:12345");

    let other_kind = SourceObjectIdentity::new(
        SourceNamespace::MilesplitAthlete,
        SourceEntityKind::Schools,
        "12345",
        "sch:wi-abbotsford",
    );
    assert_ne!(
        first.id, other_kind.id,
        "the same provider id in another table is another object"
    );
}

#[test]
fn a_namespaced_provider_keeps_its_slug_in_the_row_id() {
    let timer = SourceObjectIdentity::new(
        SourceNamespace::TimerMeet {
            provider: "wayzata".to_string(),
        },
        SourceEntityKind::Meets,
        "556",
        "meet:wayzata-556",
    );
    assert_eq!(timer.id, "timer_meet:wayzata:meets:556");
}

#[test]
fn a_merge_row_keeps_both_sides_of_the_decision_it_records() {
    let merge = CanonicalMerge::new("sch_a", "sch_b", SourceEntityKind::Schools)
        .with_retired_key("Madison West (WI)")
        .with_surviving_key("West High (WI)")
        .with_sources("milesplit_school:wi-madison-west", "wiaa:1234")
        .with_rationale("School identity: one state, one normalized name")
        .with_decided_on("2026-09-22");

    assert_eq!(merge.id, "schools:sch_a");
    assert_eq!(merge.entity, SourceEntityKind::Schools);
    assert_eq!(merge.surviving_id, "sch_b");
    assert_eq!(
        merge.retired_key, "Madison West (WI)",
        "both sides' material is kept: a bare id redirect cannot be reversed"
    );
    assert_eq!(merge.retired_sources, "milesplit_school:wi-madison-west");
    assert_eq!(
        merge.rationale,
        "School identity: one state, one normalized name"
    );
    assert_eq!(
        CanonicalMerge::new("sch_a", "sch_c", SourceEntityKind::Schools).id,
        merge.id,
        "one row per retired id: a re-derivation replaces the row for the id it retires"
    );
}
