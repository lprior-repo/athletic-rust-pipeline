use crate::{Entity, Store, Table};
use census_domain::model::{
    CanonicalEvent, CanonicalMeet, EventKind, Evidence, Gender, SourceEventLabel, SourceRef,
};
use census_domain::UsJurisdiction;

fn retained(label: &str) -> CanonicalEvent {
    let meet = CanonicalMeet::mint(
        Some(UsJurisdiction::Alabama),
        "2026-04-25",
        "Captured Section Meet",
        None,
    );
    let mut event = CanonicalEvent::new(
        &meet,
        EventKind::Unmapped {
            label: label.into(),
        },
        Gender::Boys,
        Some("2A"),
        Some("preliminaries"),
    );
    let source = SourceRef::new(
        "milesplit_al",
        Some("https://al.milesplit.com/meets/745082/results".into()),
    );
    event.source_labels.push(SourceEventLabel {
        source: source.clone(),
        label: label.into(),
    });
    event.evidence.push(Evidence::parsed(source, "2026-09-28"));
    event
}

#[test]
fn source_proven_refinement_preserves_event_references_in_both_orders() {
    let historical = retained("Boys 2A 110m Hurdles Preliminaries");
    let mut corrected = historical.clone();
    corrected.kind = historical
        .resolved_source_kind()
        .expect("published hurdles event");
    assert_eq!(corrected.kind, EventKind::Track110mHurdles);
    let mut forward = historical.clone();
    forward.merge(corrected.clone());
    let mut reverse = corrected.clone();
    reverse.merge(historical.clone());
    assert_eq!(forward, corrected);
    assert_eq!(reverse, corrected);
    assert_eq!(forward.id, historical.id);
    assert_eq!(forward.meet, historical.meet);
    assert_eq!(forward.division, historical.division);
    assert_eq!(forward.round, historical.round);
    assert_eq!(forward.source_labels, historical.source_labels);
    assert_eq!(forward.evidence, historical.evidence);
}

#[test]
fn unsupported_unbound_and_conflicting_labels_cannot_refine() {
    assert_eq!(retained("Boys Mystery Race").resolved_source_kind(), None);
    let mut unbound = retained("Boys 110m Hurdles");
    unbound.evidence.clear();
    assert_eq!(unbound.resolved_source_kind(), None);
    let mut conflicting = retained("Boys 110m Hurdles");
    let source = conflicting.source_labels[0].source.clone();
    conflicting.source_labels.push(SourceEventLabel {
        source,
        label: "Boys 100m Dash".into(),
    });
    assert_eq!(conflicting.resolved_source_kind(), None);
}

#[test]
fn refinement_cannot_change_context_or_override_known_kind() {
    let original = retained("Boys 2A 110m Hurdles Preliminaries");
    for change in 0..3 {
        let mut incoming = original.clone();
        incoming.kind = EventKind::Track110mHurdles;
        match change {
            0 => incoming.round = Some("finals".into()),
            1 => incoming.gender = Gender::Girls,
            _ => incoming.kind = EventKind::Track100m,
        }
        let mut held = original.clone();
        held.merge(incoming);
        assert_eq!(held.kind, original.kind);
        assert_eq!(held.round, original.round);
        assert_eq!(held.gender, original.gender);
        assert_eq!(held.retained_conflicts.len(), 1);
    }
    let mut known = original.clone();
    known.kind = EventKind::Track100m;
    let mut different = original;
    different.kind = EventKind::Track110mHurdles;
    known.merge(different);
    assert_eq!(known.kind, EventKind::Track100m);
    assert_eq!(known.retained_conflicts.len(), 1);
}

#[test]
fn retained_observations_survive_refinement_and_store_reopen() {
    let root = tempfile::tempdir().expect("temporary store root");
    let historical = retained("Girls Long Jump Finals");
    let mut corrected = historical.clone();
    corrected.kind = historical
        .resolved_source_kind()
        .expect("published field event");
    assert_eq!(corrected.kind, EventKind::LongJump);
    {
        let store = Store::open(root.path()).expect("open store");
        store
            .append(Table::Events, &historical)
            .expect("historical observation");
        store
            .append(Table::Events, &corrected)
            .expect("refinement observation");
        store.flush().expect("persist observations");
    }
    let store = Store::open(root.path()).expect("reopen store");
    let mut observations = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::Events, |event: CanonicalEvent| {
            observations.push(event);
            Ok(())
        })
        .expect("history");
    assert!(observations.contains(&historical));
    assert!(observations.contains(&corrected));
    let merged: Vec<CanonicalEvent> = store.scan(Table::Events).expect("merged events");
    assert_eq!(merged, vec![corrected]);
}

#[test]
fn a_conflicted_typed_observation_cannot_refine_a_retained_unmapped_event() {
    let original = retained("Boys 2A 110m Hurdles Preliminaries");
    let mut incoming = original.clone();
    incoming.kind = EventKind::Track110mHurdles;
    let mut conflicting = incoming.clone();
    conflicting.kind = EventKind::Track100m;
    incoming.merge(conflicting);
    let conflicts = incoming.retained_conflicts.clone();
    let mut held = original.clone();
    held.merge(incoming);
    assert_eq!(held.kind, original.kind);
    assert_eq!(held.source_labels, original.source_labels);
    assert_eq!(held.evidence, original.evidence);
    assert!(conflicts
        .iter()
        .all(|conflict| held.retained_conflicts.contains(conflict)));
}

#[test]
fn typed_side_contradictory_or_unbound_labels_cannot_bypass_event_collision() {
    let original = retained("Boys 2A 110m Hurdles Preliminaries");
    for contradictory in [true, false] {
        let mut incoming = original.clone();
        incoming.kind = EventKind::Track110mHurdles;
        if contradictory {
            incoming.source_labels[0].label = "Boys 100m Dash".into();
        } else {
            incoming.evidence.clear();
        }
        for (mut held, other) in [
            (original.clone(), incoming.clone()),
            (incoming.clone(), original.clone()),
        ] {
            let prior = held.clone();
            held.merge(other);
            assert_eq!(held.kind, prior.kind);
            assert_eq!(held.source_labels, prior.source_labels);
            assert_eq!(held.evidence, prior.evidence);
            assert_eq!(held.retained_conflicts.len(), 1);
        }
    }
}
