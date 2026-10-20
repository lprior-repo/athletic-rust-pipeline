use super::*;

#[test]
fn rejected_member_withdrawal_preserves_original_authority_head_and_physical_rows() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let source = fixture::install(&fixture, "22222", "77", Sport::OutdoorTrack, true)?;
    fixture.accept(&source.athlete.id)?;
    let original_approval = issuance::review_revision(
        &fixture,
        &source,
        "controlled-guards-original-source",
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let original_digest = super::super::super::hash(&original_approval);
    let before_physical = super::physical(&fixture.store)?;
    let before_head = super::authority_head(&fixture.store)?;
    let (before, _) = fixture.assess()?;
    check!(eq; before.resolution(&source.athlete.id).school(), Some(&source.school.id));
    let witness = super::source_witness(&fixture.store, "controlled-guards-original-source")?;
    let prepared = fixture.store.prepare_roster_member_withdrawal_v1(
        RosterAuthorityTextV1::parse("controlled-guards-original-source")?,
        witness,
        RosterMemberWithdrawalReviewV1 {
            operation: RosterAuthorityTextV1::parse("controlled-guards-rejected-withdrawal")?,
            actor: RosterReviewActorV1::LocalOperator,
            reviewed_at: fixture::instant()?,
            rationale: RosterAuthorityTextV1::parse(
                "Controlled negative case: intentionally wrong approval bytes",
            )?,
            scope: ProfileDateRangeV1::new(
                ProfileDate::parse("2026-10-16")?,
                ProfileDate::parse("2026-10-16")?,
            )?,
        },
        AttachmentReplayGrantV1::new(),
    )?;
    let correct_approval = prepared.approval_bytes().to_vec();
    let wrong_approval = correct_approval
        .iter()
        .map(|b| b.wrapping_add(1))
        .collect::<Vec<_>>();
    let wrong_digest = "deliberately-wrong-digest-value";
    let result =
        fixture
            .store
            .apply_roster_member_withdrawal_v1(prepared, &wrong_approval, wrong_digest);
    check!(matches!(
        result,
        Err(census_store::StoreError::RosterSourceIssuance(
            census_store::RosterSourceIssuanceStoreError::ApprovalMismatch
        ))
    ));
    let after_head = super::authority_head(&fixture.store)?;
    check!(eq; after_head, before_head);
    check!(eq; super::physical(&fixture.store)?, before_physical);
    let (after, _) = fixture.assess()?;
    check!(eq; after.resolution(&source.athlete.id).school(), Some(&source.school.id));
    let replayed = fixture
        .store
        .replay_roster_source_review_v1("controlled-guards-original-source", &original_digest)?;
    check!(matches!(replayed, census_store::Application::Repeated(_)));
    let (after_replay, _) = fixture.assess()?;
    check!(eq; after_replay.resolution(&source.athlete.id).school(), Some(&source.school.id));
    Ok(())
}

#[test]
fn stale_member_withdrawal_rejected_after_intervening_source_review() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let source = fixture::install(&fixture, "22222", "77", Sport::OutdoorTrack, true)?;
    fixture.accept(&source.athlete.id)?;
    let original_approval = issuance::review_revision(
        &fixture,
        &source,
        "controlled-guards-stale-source",
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let original_digest = super::super::super::hash(&original_approval);
    let before_physical = super::physical(&fixture.store)?;
    let (before, _) = fixture.assess()?;
    check!(eq; before.resolution(&source.athlete.id).school(), Some(&source.school.id));
    let witness = super::source_witness(&fixture.store, "controlled-guards-stale-source")?;
    let stale_prepared = fixture.store.prepare_roster_member_withdrawal_v1(
        RosterAuthorityTextV1::parse("controlled-guards-stale-source")?,
        witness,
        RosterMemberWithdrawalReviewV1 {
            operation: RosterAuthorityTextV1::parse("controlled-guards-stale-withdrawal")?,
            actor: RosterReviewActorV1::LocalOperator,
            reviewed_at: fixture::instant()?,
            rationale: RosterAuthorityTextV1::parse(
                "Controlled negative: intentionally left stale by intervening source review",
            )?,
            scope: ProfileDateRangeV1::new(
                ProfileDate::parse("2026-10-16")?,
                ProfileDate::parse("2026-10-16")?,
            )?,
        },
        AttachmentReplayGrantV1::new(),
    )?;
    let stale_approval = stale_prepared.approval_bytes().to_vec();
    let stale_digest = stale_prepared.approval_digest().to_string();
    let _reconfirmed = issuance::review_revision(
        &fixture,
        &source,
        "controlled-guards-reaffirming-source",
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let (reaffirmed, _) = fixture.assess()?;
    check!(eq; reaffirmed.resolution(&source.athlete.id).school(), Some(&source.school.id));
    let head_before = super::authority_head(&fixture.store)?;
    let result = fixture.store.apply_roster_member_withdrawal_v1(
        stale_prepared,
        &stale_approval,
        &stale_digest,
    );
    check!(matches!(
        result,
        Err(census_store::StoreError::RosterSourceIssuance(
            census_store::RosterSourceIssuanceStoreError::InputMoved
        ))
    ));
    let head_after = super::authority_head(&fixture.store)?;
    check!(eq; head_after, head_before);
    check!(eq; super::physical(&fixture.store)?, before_physical);
    let (after, _) = fixture.assess()?;
    check!(eq; after.resolution(&source.athlete.id).school(), Some(&source.school.id));
    let replayed = fixture
        .store
        .replay_roster_source_review_v1("controlled-guards-stale-source", &original_digest)?;
    check!(matches!(replayed, census_store::Application::Repeated(_)));
    let (after_replay, _) = fixture.assess()?;
    check!(eq; after_replay.resolution(&source.athlete.id).school(), Some(&source.school.id));
    Ok(())
}
