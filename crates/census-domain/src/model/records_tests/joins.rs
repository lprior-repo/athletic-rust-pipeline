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
