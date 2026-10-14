use super::fixture::*;
use super::*;

#[test]
fn empty_history_requires_the_exact_genesis_head() -> TestResult {
    let fixture = Fixture::new()?;
    check!(eq; fixture.project()?.head(), &fixture.head);
    let wrong = RegistryHead {
        sequence: 1,
        digest: "a".repeat(64),
    };
    check!(eq; replay_candidate_history(&fixture.installation, &wrong, &[], ReplayLimits::SMALL).err(), Some(HistoryError::Head));
    Ok(())
}

#[test]
fn a_duplicate_or_skipped_sequence_is_corruption() -> TestResult {
    for sequence in [0, 2, u64::MAX] {
        let mut fixture = Fixture::new()?;
        let _births = fixture.append(establish("a", "1"))?;
        fixture
            .records
            .first_mut()
            .ok_or("missing record")?
            .sequence = sequence;
        check!(eq; fixture.project().err(), Some(HistoryError::Sequence));
    }
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("a", "1"))?;
    let duplicate = fixture.records.first().ok_or("missing record")?.clone();
    fixture.records.push(duplicate);
    check!(eq; fixture.project().err(), Some(HistoryError::Sequence));
    Ok(())
}

#[test]
fn payload_changes_are_not_accepted_under_the_same_history_digest() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _births = fixture.append(establish("a", "1"))?;
    let record = fixture.records.first_mut().ok_or("missing record")?;
    if let HistoryRequest::Establish { binding, .. } = &mut record.request {
        binding.source.identifier = "different-source-id".into();
    }
    check!(eq; fixture.project().err(), Some(HistoryError::Payload));
    Ok(())
}

#[test]
fn previous_head_and_expected_final_head_are_independent_checks() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _births = fixture.append(establish("a", "1"))?;
    let mut wrong_final = fixture.head.clone();
    wrong_final.digest = "a".repeat(64);
    check!(eq; replay_candidate_history(&fixture.installation, &wrong_final, &fixture.records, ReplayLimits::SMALL).err(), Some(HistoryError::Head));
    fixture
        .records
        .first_mut()
        .ok_or("missing record")?
        .previous
        .digest = "a".repeat(64);
    check!(eq; fixture.project().err(), Some(HistoryError::Head));
    Ok(())
}

#[test]
fn unsupported_version_and_wrong_installation_are_refused() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _births = fixture.append(establish("a", "1"))?;
    fixture.records.first_mut().ok_or("missing record")?.version = 2;
    check!(eq; fixture.project().err(), Some(HistoryError::Version));
    let record = fixture.records.first_mut().ok_or("missing record")?;
    record.version = 1;
    record.installation = crate::model::StoreIdentity::parse(&"2".repeat(64))?;
    check!(eq; fixture.project().err(), Some(HistoryError::Installation));
    Ok(())
}

#[test]
fn committed_order_cannot_be_replaced_by_record_enumeration_order() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("a", "1"))?;
    let _second = fixture.append(establish("b", "2"))?;
    fixture.records.reverse();
    check!(eq; fixture.project().err(), Some(HistoryError::Sequence));
    Ok(())
}

#[test]
fn one_application_reference_cannot_allocate_twice() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _first = fixture.append(establish("a", "1"))?;
    let _second = fixture.append(HistoryRequest::Establish {
        binding: binding("b", "2"),
        evidence: evidence("source:a", CandidateEvidenceKind::SourceBound, &["b"]),
    })?;
    check!(eq; fixture.project().err(), Some(HistoryError::Duplicate("application")));
    Ok(())
}

#[test]
fn preparation_is_repeatable_but_never_claims_a_commit() -> TestResult {
    let fixture = Fixture::new()?;
    let first = prepare_next(
        &fixture.installation,
        &fixture.head,
        establish("a", "1"),
        ReplayLimits::SMALL,
    )?;
    let second = prepare_next(
        &fixture.installation,
        &fixture.head,
        establish("a", "1"),
        ReplayLimits::SMALL,
    )?;
    check!(eq; first, second);
    check!(eq; fixture.project()?.head().sequence, 0);
    check!(eq; first.uncommitted_record.sequence, 1);
    Ok(())
}

#[test]
fn establishment_counter_exhaustion_refuses_without_a_new_id() -> TestResult {
    let fixture = Fixture::new()?;
    let exhausted = RegistryHead {
        sequence: u64::MAX,
        digest: "a".repeat(64),
    };
    check!(eq; prepare_next(&fixture.installation, &exhausted, establish("a", "1"), ReplayLimits::SMALL).err(), Some(HistoryError::CounterOverflow));
    Ok(())
}

#[test]
fn record_identity_binding_and_text_limits_refuse_exactly() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _births = fixture.append(establish("a", "1"))?;
    for (limits, expected) in [
        (
            ReplayLimits {
                records: 0,
                ..ReplayLimits::SMALL
            },
            "records",
        ),
        (
            ReplayLimits {
                identities: 0,
                ..ReplayLimits::SMALL
            },
            "identities",
        ),
        (
            ReplayLimits {
                bindings: 0,
                ..ReplayLimits::SMALL
            },
            "bindings",
        ),
        (
            ReplayLimits {
                text_bytes: 1,
                ..ReplayLimits::SMALL
            },
            "text bytes",
        ),
        (
            ReplayLimits {
                total_text_bytes: 1,
                ..ReplayLimits::SMALL
            },
            "total text bytes",
        ),
        (
            ReplayLimits {
                members_per_operation: 0,
                ..ReplayLimits::SMALL
            },
            "evidence members",
        ),
    ] {
        check!(eq; replay_candidate_history(&fixture.installation, &fixture.head, &fixture.records, limits).err(), Some(HistoryError::Limit(expected)));
    }
    Ok(())
}

#[test]
fn traversal_limit_is_checked_without_following_an_unbounded_chain() -> TestResult {
    let fixture = chain()?;
    let limits = ReplayLimits {
        traversal_steps: 1,
        ..ReplayLimits::SMALL
    };
    check!(eq; replay_candidate_history(&fixture.installation, &fixture.head, &fixture.records, limits).err(),
        Some(HistoryError::Limit("traversal steps")));
    Ok(())
}

#[test]
fn oversized_expected_operation_is_refused_before_hashing() -> TestResult {
    let fixture = Fixture::new()?;
    let mut request = establish("a", "1");
    if let HistoryRequest::Establish { binding, .. } = &mut request {
        binding.source.identifier = "x".repeat(513);
    }
    check!(eq; prepare_next(&fixture.installation, &fixture.head, request, ReplayLimits::SMALL).err(), Some(HistoryError::Limit("text bytes")));
    Ok(())
}
