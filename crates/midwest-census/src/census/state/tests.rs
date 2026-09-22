//! The lattice's rules, on values only: no store, no network, no clock.

use super::*;

/// Evidence that satisfies every §70 item: 51 jurisdictions, a workbook that carries the cohort, and
/// retained findings that are non-zero on purpose (findings never block a seal).
fn evidence() -> SealEvidence {
    SealEvidence {
        open: OpenWork::default(),
        counts: SealCounts {
            jurisdictions: 51,
            schools: 18_047,
            meets: 11_007,
            athletes: 1_226_212,
            class_of_2027: 307_653,
            performances: 4_100_000,
            coaches: 27_580,
        },
        retained: RetainedFindings {
            gaps: vec![
                GapTally {
                    class: "missing_coach".to_string(),
                    unit: "athletes".to_string(),
                    count: 264_452,
                },
                GapTally {
                    class: "missing_profile".to_string(),
                    unit: "athletes".to_string(),
                    count: 13_202,
                },
            ],
            conflicts: 11_342,
            retry_exhausted: 96,
            source_failures: 4,
            observations: 9_800_000,
            calculations: 4_060_000,
        },
        workbook: WorkbookCheck {
            sheets: 24,
            rows: 4_407_653,
            digests: vec!["0f1e".to_string()],
            mapped_athletes: 307_653,
            counts_reconciled: true,
            coverage_reconciled: true,
            metrics_reconciled: true,
            export_verified: true,
            discrepancies: Vec::new(),
        },
        observed_on: "2026-09-22".to_string(),
    }
}

/// The state every seal test starts from: the phase a census must be in to complete.
fn exporting() -> CensusState {
    CensusState::Exporting
}

/// Seal any state. A refusal leaves the state where it was, so the caller sees the lattice it
/// started from rather than a half-completed one.
fn seal_from(mut state: CensusState, evidence: SealEvidence) -> Result<CensusState, SealError> {
    let outcome = state.seal(evidence);
    outcome.map(|()| state)
}

/// Seal from the export phase: the phase a census must reach before it can complete.
fn seal_from_export(evidence: SealEvidence) -> Result<CensusState, SealError> {
    seal_from(exporting(), evidence)
}

/// Advance a copy of `state`, so a refused advance is visible as an error and the original state
/// survives to be probed again.
fn advance_of(state: &CensusState, to: Phase) -> Result<CensusState, SealError> {
    let mut probe = state.clone();
    probe.advance(to).map(|()| probe)
}

/// Walk the lattice from discovery to the phase before completion.
fn walk_to(phase: Phase) -> CensusState {
    let mut state = CensusState::Discovering;
    while state.phase() != phase {
        let next = state.phase().next().expect("every open phase has a next");
        state.advance(next).expect("one-step advance is legal");
    }
    state
}

#[test]
fn the_lattice_advances_one_phase_at_a_time_from_discovery_to_export() {
    let mut state = CensusState::Discovering;
    for phase in Phase::ALL.iter().copied() {
        assert_eq!(state.phase(), phase, "the state is at {phase:?}");
        let Some(next) = phase.next() else {
            break;
        };
        state.advance(next).expect("one-step advance is legal");
        assert_eq!(state.phase().ordinal(), phase.ordinal() + 1);
    }
    assert_eq!(state.phase(), Phase::Exporting);
    assert!(state.sealed().is_none());
    assert!(Phase::Exporting.next().is_none());
}

#[test]
fn acquiring_cannot_reach_complete_or_skip_a_phase() {
    let acquiring = walk_to(Phase::Acquiring);
    let refused = advance_of(&acquiring, Phase::Complete);
    assert!(matches!(
        refused,
        Err(SealError::OutOfOrder {
            from: "acquiring",
            to: "complete"
        })
    ));

    let skipped = advance_of(&acquiring, Phase::Exporting);
    assert!(matches!(
        skipped,
        Err(SealError::OutOfOrder {
            from: "acquiring",
            to: "exporting"
        })
    ));

    let repeated = advance_of(&acquiring, Phase::Acquiring);
    assert!(matches!(
        repeated,
        Err(SealError::OutOfOrder {
            from: "acquiring",
            to: "acquiring"
        })
    ));
}

#[test]
fn a_seal_is_refused_before_the_export_phase() {
    let reviewing = walk_to(Phase::Reviewing);
    let refused = seal_from(reviewing, evidence());
    assert!(
        matches!(
            refused,
            Err(SealError::OutOfOrder {
                from: "reviewing",
                to: "complete"
            })
        ),
        "a census that has not exported cannot complete"
    );
}

#[test]
fn every_open_decision_refuses_the_seal_by_name() {
    let cases = [
        (
            OpenWork {
                jurisdiction_sweeps: 3,
                ..OpenWork::default()
            },
            AcceptanceItem::JurisdictionSweepsTerminal,
        ),
        (
            OpenWork {
                source_objects: 41,
                ..OpenWork::default()
            },
            AcceptanceItem::SourceObjectsTerminal,
        ),
        (
            OpenWork {
                cohort_decisions: 7,
                ..OpenWork::default()
            },
            AcceptanceItem::CohortDecisionsTerminal,
        ),
        (
            OpenWork {
                identity_candidates: 2,
                ..OpenWork::default()
            },
            AcceptanceItem::IdentityCandidatesTerminal,
        ),
    ];
    for (open, item) in cases {
        let mut packet = evidence();
        packet.open = open;
        assert_eq!(packet.open_items(), vec![item]);
        let refused = seal_from_export(packet);
        assert!(
            matches!(refused, Err(SealError::ItemUnmet { item: named, .. }) if named == item),
            "the refusal must name {item:?}"
        );
    }
}

#[test]
fn a_census_that_claims_work_but_proves_nothing_is_refused() {
    let mut no_evidence = evidence();
    no_evidence.retained.observations = 0;
    no_evidence.retained.calculations = 0;
    assert_eq!(
        no_evidence.open_items(),
        vec![
            AcceptanceItem::EvidenceDurable,
            AcceptanceItem::CalculationsReproducible
        ]
    );
    assert!(matches!(
        seal_from_export(no_evidence),
        Err(SealError::ItemUnmet {
            item: AcceptanceItem::EvidenceDurable,
            ..
        })
    ));

    // An empty census makes no such claim, so the same zero counts are not a refusal.
    let mut empty = evidence();
    empty.counts.athletes = 0;
    empty.counts.performances = 0;
    empty.counts.class_of_2027 = 0;
    empty.retained.observations = 0;
    empty.retained.calculations = 0;
    empty.workbook.mapped_athletes = 0;
    assert_eq!(empty.open_items(), Vec::new());
    assert!(seal_from_export(empty).is_ok());
}

#[test]
fn an_unreconciled_workbook_refuses_the_seal() {
    let cases = [
        (
            WorkbookCheck {
                counts_reconciled: false,
                ..evidence().workbook
            },
            AcceptanceItem::WorkbookCountsReconcile,
        ),
        (
            WorkbookCheck {
                coverage_reconciled: false,
                ..evidence().workbook
            },
            AcceptanceItem::CoverageReportReconciles,
        ),
        (
            WorkbookCheck {
                metrics_reconciled: false,
                ..evidence().workbook
            },
            AcceptanceItem::RunMetricsReconcile,
        ),
        (
            WorkbookCheck {
                export_verified: false,
                ..evidence().workbook
            },
            AcceptanceItem::ExportVerified,
        ),
        (
            WorkbookCheck {
                discrepancies: vec!["Summary!B7 1,226,212 != store 1,226,211".to_string()],
                ..evidence().workbook
            },
            AcceptanceItem::ExportVerified,
        ),
    ];
    for (workbook, item) in cases {
        let mut packet = evidence();
        packet.workbook = workbook;
        assert_eq!(packet.open_items(), vec![item]);
        assert!(matches!(
            seal_from_export(packet),
            Err(SealError::ItemUnmet { item: named, .. }) if named == item
        ));
    }
}

#[test]
fn the_workbook_must_carry_every_cohort_athlete() {
    let mut missing = evidence();
    missing.workbook.mapped_athletes = missing.counts.class_of_2027 - 1;
    assert_eq!(missing.open_items(), vec![AcceptanceItem::WorkbookMapped]);

    let mut complete = evidence();
    complete.workbook.mapped_athletes = complete.counts.class_of_2027;
    assert_eq!(complete.open_items(), Vec::new());
}

#[test]
fn retained_findings_do_not_block_a_seal_and_travel_inside_it() {
    let packet = evidence();
    assert!(packet.retained.gaps.iter().any(|gap| gap.count > 0));
    assert!(packet.retained.conflicts > 0 && packet.retained.retry_exhausted > 0);
    let sealed = seal_from_export(packet.clone()).expect("findings never block a seal");
    let seal = sealed.sealed().expect("the state is complete");

    assert_eq!(seal.counts.class_of_2027, 307_653);
    assert_eq!(seal.retained.conflicts, 11_342);
    assert_eq!(seal.retained.gaps.len(), 2);
    assert_eq!(seal.sealed_on, "2026-09-22");
    assert_eq!(seal.workbook_rows, packet.workbook.rows);
    assert_eq!(sealed.phase(), Phase::Complete);
}

#[test]
fn the_seal_digest_is_stable_and_moves_with_the_counts() {
    let first = seal_from_export(evidence()).expect("seals");
    let again = seal_from_export(evidence()).expect("seals");
    let (Some(a), Some(b)) = (first.sealed(), again.sealed()) else {
        panic!("both states must be sealed");
    };
    assert_eq!(
        a.digest, b.digest,
        "identical evidence renders identical bytes"
    );
    assert_eq!(a.digest.len(), 64, "sha256 hex");

    // Gap order is not evidence: the digest sorts the tallies before rendering them.
    let mut reordered = evidence();
    reordered.retained.gaps.reverse();
    let reordered = seal_from_export(reordered).expect("seals");
    assert_eq!(
        reordered.sealed().map(|seal| seal.digest.clone()),
        Some(a.digest.clone())
    );

    // One more athlete is different evidence.
    let mut moved = evidence();
    moved.counts.athletes += 1;
    let moved = seal_from_export(moved).expect("seals");
    assert_ne!(
        moved.sealed().map(|seal| seal.digest.clone()),
        Some(a.digest.clone())
    );
}

#[test]
fn sealing_twice_returns_the_same_seal() {
    let once = seal_from_export(evidence()).expect("seals");
    let mut twice = once.clone();
    twice.seal(evidence()).expect("seals again");
    assert_eq!(once, twice, "a sealed census does not mint a second digest");
}

/// The digest one sealed state carries.
fn digest_of(state: &CensusState) -> String {
    state.sealed().expect("the state is sealed").digest.clone()
}

#[test]
fn the_seal_binds_the_workbook_it_certifies() {
    let base = digest_of(&seal_from_export(evidence()).expect("seals"));

    // The same counts, a different export: a seal that cannot tell these apart certifies neither.
    let mut reexported = evidence();
    reexported.workbook.digests = vec!["9a9a9a".to_string()];
    assert_ne!(
        base,
        digest_of(&seal_from_export(reexported).expect("seals"))
    );

    let mut resheeted = evidence();
    resheeted.workbook.sheets = evidence().workbook.sheets + 1;
    assert_ne!(
        base,
        digest_of(&seal_from_export(resheeted).expect("seals"))
    );

    // Workbook digests arrive in whatever order the filesystem lists them.
    let mut reordered = evidence();
    reordered.workbook.digests = vec!["ff".to_string(), "00".to_string()];
    let mut sorted = evidence();
    sorted.workbook.digests = vec!["00".to_string(), "ff".to_string()];
    assert_eq!(
        digest_of(&seal_from_export(reordered).expect("seals")),
        digest_of(&seal_from_export(sorted).expect("seals")),
        "the digest is over the set of workbook bytes, not their order"
    );
}
