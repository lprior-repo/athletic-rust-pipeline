//! The lattice's rules, on values only: no store, no network, no clock.

use census_domain::model::{
    ReviewCase, ReviewState, COHORT_DECISION_FAMILIES, COHORT_IDENTITY_CONFIDENCE_FAMILY,
    COHORT_UNVERIFIED_FAMILY, UNRESOLVED_VENUE_FAMILY,
};

use sha2::{Digest, Sha256};

use super::*;

/// Evidence that satisfies every §70 item: 51 jurisdiction buckets, a workbook that carries the
/// cohort, and retained findings that are non-zero on purpose (findings never block a seal).
fn evidence() -> SealEvidence {
    SealEvidence {
        open: OpenWork {
            jurisdiction_sweeps: Some(0),
            source_objects: Some(0),
            cohort_decisions: Some(0),
            identity_candidates: Some(0),
        },
        counts: SealCounts {
            jurisdiction_buckets: 51,
            schools: 18_047,
            meets: 11_007,
            athletes: 1_226_212,
            class_of_2027: 307_653,
            cohort_performances: 4_100_000,
            coaches: 27_580,
        },
        retained: RetainedFindings {
            silent_sources: vec!["wayzata_mn".to_string(), "wiaa_results_wi".to_string()],
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
            access_conditions: 96,
            blocked_hosts: 60,
            throttled_hosts: 36,
            source_failures: Some(4),
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
                jurisdiction_sweeps: Some(3),
                source_objects: Some(0),
                cohort_decisions: Some(0),
                identity_candidates: Some(0),
            },
            AcceptanceItem::JurisdictionSweepsTerminal,
        ),
        (
            OpenWork {
                jurisdiction_sweeps: Some(0),
                source_objects: Some(41),
                cohort_decisions: Some(0),
                identity_candidates: Some(0),
            },
            AcceptanceItem::SourceObjectsTerminal,
        ),
        (
            OpenWork {
                jurisdiction_sweeps: Some(0),
                source_objects: Some(0),
                cohort_decisions: Some(7),
                identity_candidates: Some(0),
            },
            AcceptanceItem::CohortDecisionsTerminal,
        ),
        (
            OpenWork {
                jurisdiction_sweeps: Some(0),
                source_objects: Some(0),
                cohort_decisions: Some(0),
                identity_candidates: Some(2),
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

/// A measurement nobody took is not a zero.
///
/// The store path of `census-service seal` cannot read the workflow journal, so it reports those
/// fields as `None`. It used to report `0`, and the seal certified §70 items it had never checked -
/// the exact failure this rule exists to make impossible.
#[test]
fn unmeasured_open_work_refuses_the_seal_by_name() {
    let cases = [
        (
            OpenWork {
                jurisdiction_sweeps: None,
                source_objects: Some(0),
                cohort_decisions: Some(0),
                identity_candidates: Some(0),
            },
            AcceptanceItem::JurisdictionSweepsTerminal,
        ),
        (
            OpenWork {
                jurisdiction_sweeps: Some(0),
                source_objects: None,
                cohort_decisions: Some(0),
                identity_candidates: Some(0),
            },
            AcceptanceItem::SourceObjectsTerminal,
        ),
        (
            OpenWork {
                jurisdiction_sweeps: Some(0),
                source_objects: Some(0),
                cohort_decisions: None,
                identity_candidates: Some(0),
            },
            AcceptanceItem::CohortDecisionsTerminal,
        ),
        (
            OpenWork {
                jurisdiction_sweeps: Some(0),
                source_objects: Some(0),
                cohort_decisions: Some(0),
                identity_candidates: None,
            },
            AcceptanceItem::IdentityCandidatesTerminal,
        ),
    ];
    for (open, item) in cases {
        let mut packet = evidence();
        packet.open = open;
        assert_eq!(packet.open_items(), vec![item]);
        assert!(
            packet.detail(item).contains("not measured"),
            "an unmeasured refusal must say so rather than print a number: {}",
            packet.detail(item)
        );
        let refused = seal_from_export(packet);
        assert!(
            matches!(refused, Err(SealError::ItemUnmet { item: named, .. }) if named == item),
            "the refusal must name {item:?}"
        );
    }
}

/// The default is *unmeasured*, not terminal: a caller that forgets a field gets a refusal instead
/// of a seal, and all four items are named at once.
#[test]
fn the_default_open_work_is_unmeasured_and_refuses() {
    let mut packet = evidence();
    packet.open = OpenWork::default();
    assert_eq!(
        packet.open_items().len(),
        4,
        "every field of the default is unmeasured, so every open-work item stays open"
    );
    assert!(
        seal_from_export(packet).is_err(),
        "an unmeasured seal must not be granted"
    );
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

    let mut empty = evidence();
    empty.counts.athletes = 0;
    empty.counts.cohort_performances = 0;
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
    assert!(packet.retained.conflicts > 0 && packet.retained.access_conditions > 0);
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

    let mut reordered = evidence();
    reordered.retained.gaps.reverse();
    let reordered = seal_from_export(reordered).expect("seals");
    assert_eq!(
        reordered.sealed().map(|seal| seal.digest.clone()),
        Some(a.digest.clone())
    );

    let mut moved = evidence();
    moved.counts.athletes += 1;
    let moved = seal_from_export(moved).expect("seals");
    assert_ne!(
        moved.sealed().map(|seal| seal.digest.clone()),
        Some(a.digest.clone())
    );
}

/// The digest body is a wire format, not an implementation detail: a stored digest is re-rendered
/// from the same evidence and compared, so every field's name, order and separator is a promise to
/// the seals already written. The body below is `seal_digest.rs`'s format string transcribed field
/// by field, so a field that is moved, renamed or dropped fails here with both strings side by side
/// instead of as one opaque hash that cannot say which field moved.
#[test]
fn the_digest_is_pinned_field_by_field() {
    let body = "census-seal-v6\n\
         jurisdiction_buckets=51\n\
         schools=18047\n\
         meets=11007\n\
         athletes=1226212\n\
         co2027=307653\n\
         cohort_performances=4100000\n\
         coaches=27580\n\
         conflicts=11342\n\
         access_conditions=96\n\
         blocked_hosts=60\n\
         throttled_hosts=36\n\
         silent_sources=wayzata_mn|wiaa_results_wi\n\
         source_failures=4\n\
         observations=9800000\n\
         calculations=4060000\n\
         workbook_rows=4407653\n\
         workbook_sheets=24\n\
         workbook_sha256=0f1e\n\
         gaps=missing_coach:athletes:264452|missing_profile:athletes:13202\n";
    let mut hasher = Sha256::new();
    hasher.update(body.as_bytes());
    assert_eq!(
        super::seal_digest::render(&evidence()),
        format!("{:x}", hasher.finalize()),
        "the rendered digest is the sha256 of exactly that body"
    );
    assert_eq!(
        digest_of(&seal_from_export(evidence()).expect("seals")),
        "62820209dbeaa3afbc6ca9546b27c0206d4f574624173c8c201e8cb09710fe36",
        "the sealed digest is that digest in lowercase hex, which is what a stored `seal.json` carries"
    );

    let mut named = evidence();
    named.retained.silent_sources.push("wayzata_ia".to_string());
    assert_ne!(
        super::seal_digest::render(&named),
        super::seal_digest::render(&evidence()),
        "a source object that finished empty is part of what the digest identifies"
    );

    let mut unmeasured = evidence();
    unmeasured.retained.source_failures = None;
    assert_ne!(
        super::seal_digest::render(&unmeasured),
        super::seal_digest::render(&evidence()),
        "an unmeasured count has its own spelling in the digest"
    );
}

/// A seal written before the renames carries its state-rollup and performance counts under the old
/// names. A recorded seal is reported rather than re-derived, so an old `seal.json` has to read: each
/// alias is the promise, and the digest it carries stays the digest it was written with.
#[test]
fn a_seal_counted_before_the_rename_still_reads() {
    let recorded = r#"{"jurisdictions":51,"schools":18047,"meets":11007,"athletes":1226212,
                       "class_of_2027":307653,"performances":4100000,"coaches":27580}"#;
    let counts: SealCounts =
        serde_json::from_str(recorded).expect("a seal.json written under the old names reads");
    assert_eq!(counts.jurisdiction_buckets, 51);
    assert_eq!(counts.cohort_performances, 4_100_000);
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

/// A jurisdiction that recorded every stage and left no roster behind is terminal; one that is
/// missing a stage, or that owes rosters a host refused, is not.
#[test]
fn a_jurisdiction_is_terminal_only_when_every_stage_ran() {
    let complete = JurisdictionStages {
        teams: true,
        rosters: true,
        meets: true,
        owed_rosters: 0,
    };
    assert!(complete.terminal());
    for partial in [
        JurisdictionStages {
            teams: false,
            ..complete
        },
        JurisdictionStages {
            rosters: false,
            ..complete
        },
        JurisdictionStages {
            meets: false,
            ..complete
        },
        JurisdictionStages {
            owed_rosters: 12,
            ..complete
        },
    ] {
        assert!(!partial.terminal(), "{partial:?} still owes work");
    }
}

/// The names an operator reads are the pieces the object has no outcome for, in the order it runs
/// them — so `open-work` says what the next run would do, not merely that something is owed.
#[test]
fn owing_names_each_unfinished_piece_in_run_order() {
    assert_eq!(
        JurisdictionStages::default().owing(),
        vec!["teams", "rosters", "meets"]
    );
    let blocked = JurisdictionStages {
        teams: true,
        rosters: true,
        meets: true,
        owed_rosters: 3,
    };
    assert_eq!(blocked.owing(), vec!["blocked rosters"]);
}

/// A read that failed is not a sweep that finished, and an object that never ran is not one that
/// owes nothing: both carry a record with no stage outcome, and both count.
#[test]
fn unread_jurisdictions_count_as_owing() {
    let terminal = JurisdictionStages {
        teams: true,
        rosters: true,
        meets: true,
        owed_rosters: 0,
    };
    assert_eq!(owed_jurisdictions(&[terminal]), 0);
    assert_eq!(
        owed_jurisdictions(&[terminal, JurisdictionStages::default(), terminal]),
        1
    );
}

/// A source object is owed until it has accepted an observation or completed a window: the first
/// says its rows are durable, the second that the walk which owns it finished posting.
#[test]
fn a_source_object_is_owed_until_it_accepts_an_observation_or_completes_a_window() {
    let written = SourceObject {
        endpoint: "milesplit_wi".to_string(),
        observations: 1,
        windows: 0,
    };
    let resumed = SourceObject {
        endpoint: "wayzata_mn".to_string(),
        observations: 0,
        windows: 1,
    };
    let empty_read = SourceObject {
        endpoint: "wiaa_results_wi".to_string(),
        observations: 0,
        windows: 3,
    };
    let untouched = SourceObject {
        endpoint: "never_walked".to_string(),
        observations: 0,
        windows: 0,
    };
    assert!(written.terminal());
    assert!(resumed.terminal());
    assert!(empty_read.terminal());
    assert!(!untouched.terminal());
    assert_eq!(
        owed_source_objects(&[written, resumed, empty_read, untouched]),
        1
    );
}

/// The names the owed count cannot carry: an object whose walk finished without appending a row is
/// terminal, so it is not owed — and it still has to be recorded, or "read and empty" and "never
/// read" are the same seal.
#[test]
fn the_objects_that_finished_empty_are_named_rather_than_owed() {
    let written = SourceObject {
        endpoint: "milesplit_mn".to_string(),
        observations: 412,
        windows: 1,
    };
    let resumed = SourceObject {
        endpoint: "wayzata_mn".to_string(),
        observations: 0,
        windows: 1,
    };
    let empty_read = SourceObject {
        endpoint: "wiaa_results_wi".to_string(),
        observations: 0,
        windows: 3,
    };
    let untouched = SourceObject {
        endpoint: "never_walked".to_string(),
        observations: 0,
        windows: 0,
    };
    let objects = [written, resumed, empty_read, untouched];
    assert_eq!(owed_source_objects(&objects), 1);
    assert_eq!(
        silent_source_objects(&objects),
        vec!["wayzata_mn".to_string(), "wiaa_results_wi".to_string()],
        "sorted, and only the objects whose walk finished with nothing"
    );
}

/// The state a cohort case starts in is what makes this item attainable, so the item counts only the
/// cases a later answer is actually owed for.
///
/// [`ReviewCase::minted`] is the pass's own constructor: every family the census's rules decide is
/// minted terminal, and the cohort families are two of those. A case that starts `Pending` in a cohort
/// family is therefore a decision the run genuinely owes, which is what this item is for.
#[test]
fn the_pass_mints_no_cohort_case_a_decision_is_owed_for() {
    let minted: Vec<ReviewCase> = COHORT_DECISION_FAMILIES
        .iter()
        .enumerate()
        .map(|(index, family)| {
            ReviewCase::minted(
                family,
                format!("athlete:{index}"),
                "A Runner (Somewhere High)",
                "the finding",
            )
        })
        .collect();
    assert!(
        minted
            .iter()
            .all(|case| case.state == ReviewState::Retained),
        "the rule decides these rows, so their cases start terminal: {minted:?}"
    );
    assert_eq!(
        owed_cohort_decisions(&minted),
        0,
        "a census whose cohort findings are all published at the bar their evidence supports owes no cohort decision"
    );
}

/// A cohort decision is owed while its case has no verdict, whatever put the athlete in the queue.
#[test]
fn a_pending_cohort_case_is_owed_a_decision() {
    let unverified = ReviewCase::pending(
        COHORT_UNVERIFIED_FAMILY,
        "athlete:1",
        "A Runner (Somewhere High)",
        "no cohort evidence",
    );
    let low_confidence = ReviewCase::pending(
        COHORT_IDENTITY_CONFIDENCE_FAMILY,
        "athlete:2",
        "B Runner (Somewhere High)",
        "identity confidence 65 below the high bar of 85",
    );
    assert_eq!(owed_cohort_decisions(&[unverified, low_confidence]), 2);
}

/// A verdict is the decision, including the one that declines to decide: `Retained` is the lane
/// saying the evidence does not settle it, and the case stays visible in the workbook either way.
#[test]
fn a_decided_cohort_case_is_not_owed_again() {
    let mut resolved = ReviewCase::pending(
        COHORT_UNVERIFIED_FAMILY,
        "athlete:1",
        "A Runner (Somewhere High)",
        "no cohort evidence",
    );
    resolved.state = ReviewState::Resolved;
    let mut retained = ReviewCase::pending(
        COHORT_IDENTITY_CONFIDENCE_FAMILY,
        "athlete:2",
        "B Runner (Somewhere High)",
        "identity confidence 65 below the high bar of 85",
    );
    retained.state = ReviewState::Retained;
    assert_eq!(owed_cohort_decisions(&[resolved, retained]), 0);
}

/// Every retained case without a verdict is an open identity candidate, whichever family kept it:
/// the item is about the decision, and only a verdict is one.
#[test]
fn every_pending_case_is_an_open_identity_candidate() {
    let pending = ReviewCase::pending(
        COHORT_UNVERIFIED_FAMILY,
        "athlete:1",
        "A Runner (Somewhere High)",
        "no cohort evidence",
    );
    let mut decided = ReviewCase::pending(
        UNRESOLVED_VENUE_FAMILY,
        "meet:1",
        "Some Invitational",
        "no evidence placed the venue in a jurisdiction",
    );
    decided.state = ReviewState::Retained;
    assert_eq!(owed_identity_candidates(&[pending, decided]), 1);
}

/// Another lane's open case is that lane's work, not a cohort decision: the seal counts a
/// jurisdiction question once, under the item that owns it.
#[test]
fn another_lanes_pending_case_is_not_a_cohort_decision() {
    let venue = ReviewCase::pending(
        UNRESOLVED_VENUE_FAMILY,
        "meet:1",
        "Some Invitational",
        "no evidence placed the venue in a jurisdiction",
    );
    let another_venue = ReviewCase::pending(
        UNRESOLVED_VENUE_FAMILY,
        "meet:2",
        "Another Invitational",
        "no evidence placed the venue in a jurisdiction",
    );
    assert_eq!(owed_cohort_decisions(&[venue, another_venue]), 0);
}
