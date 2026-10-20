use super::super::{fixture, issuance, TestResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{ProfileDate, ProfileDateRangeV1, Sport};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{RosterMemberWithdrawalReviewV1, RosterReviewActorV1};

#[test]
fn foreign_source_witness_is_refused_for_source_withdrawal() -> TestResult {
    let fixture = fixture::Fixture::new()?;

    let source_a = fixture::install(&fixture, "22222", "77", Sport::OutdoorTrack, true)?;
    fixture.accept(&source_a.athlete.id)?;
    let approval_a = issuance::review_revision(
        &fixture,
        &source_a,
        "isolation-witness-source-a",
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let digest_a = super::super::super::hash(&approval_a);
    let (before, _) = fixture.assess()?;
    check!(eq; before.resolution(&source_a.athlete.id).school(), Some(&source_a.school.id));

    let source_b = fixture::install(&fixture, "33333", "78", Sport::OutdoorTrack, true)?;
    fixture.accept(&source_b.athlete.id)?;
    issuance::review_revision(
        &fixture,
        &source_b,
        "isolation-witness-source-b",
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let before_head = super::authority_head(&fixture.store)?;
    let before_physical = super::physical(&fixture.store)?;

    let witness_b = super::source_witness(&fixture.store, "isolation-witness-source-b")?;
    let result = fixture.store.prepare_roster_member_withdrawal_v1(
        RosterAuthorityTextV1::parse("isolation-witness-source-a")?,
        witness_b,
        RosterMemberWithdrawalReviewV1 {
            operation: RosterAuthorityTextV1::parse("isolation-witness-foreign")?,
            actor: RosterReviewActorV1::LocalOperator,
            reviewed_at: fixture::instant()?,
            rationale: RosterAuthorityTextV1::parse(
                "Controlled negative: witness from source B offered for source A withdrawal",
            )?,
            scope: ProfileDateRangeV1::new(
                ProfileDate::parse("2026-10-16")?,
                ProfileDate::parse("2026-10-16")?,
            )?,
        },
        AttachmentReplayGrantV1::new(),
    );
    check!(matches!(
        result,
        Err(census_store::StoreError::Refused { .. })
    ));

    let after_head = super::authority_head(&fixture.store)?;
    check!(eq; after_head, before_head);
    check!(eq; super::physical(&fixture.store)?, before_physical);
    let (after, _) = fixture.assess()?;
    check!(eq; after.resolution(&source_a.athlete.id).school(), Some(&source_a.school.id));
    check!(eq; after.resolution(&source_b.athlete.id).school(), Some(&source_b.school.id));
    let replayed = fixture
        .store
        .replay_roster_source_review_v1("isolation-witness-source-a", &digest_a)?;
    check!(matches!(replayed, census_store::Application::Repeated(_)));
    Ok(())
}

#[test]
fn prepared_withdrawal_from_foreign_store_is_refused() -> TestResult {
    let store_a = fixture::Fixture::new()?;
    let source_a = fixture::install(&store_a, "22222", "77", Sport::OutdoorTrack, true)?;
    store_a.accept(&source_a.athlete.id)?;
    issuance::review_revision(
        &store_a,
        &source_a,
        "isolation-foreign-store-source-a",
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let witness_a = super::source_witness(&store_a.store, "isolation-foreign-store-source-a")?;
    let prepared = store_a.store.prepare_roster_member_withdrawal_v1(
        RosterAuthorityTextV1::parse("isolation-foreign-store-source-a")?,
        witness_a,
        RosterMemberWithdrawalReviewV1 {
            operation: RosterAuthorityTextV1::parse("isolation-foreign-store-withdrawal")?,
            actor: RosterReviewActorV1::LocalOperator,
            reviewed_at: fixture::instant()?,
            rationale: RosterAuthorityTextV1::parse(
                "Controlled: withdrawal prepared on store A, applied to store B",
            )?,
            scope: ProfileDateRangeV1::new(
                ProfileDate::parse("2026-10-16")?,
                ProfileDate::parse("2026-10-16")?,
            )?,
        },
        AttachmentReplayGrantV1::new(),
    )?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_string();

    let store_b = fixture::Fixture::new()?;
    let source_b = fixture::install(&store_b, "22222", "78", Sport::OutdoorTrack, true)?;
    store_b.accept(&source_b.athlete.id)?;
    issuance::review_revision(
        &store_b,
        &source_b,
        "isolation-foreign-store-source-b",
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let before_physical = super::physical(&store_b.store)?;
    let (before, _) = store_b.assess()?;
    check!(eq; before.resolution(&source_b.athlete.id).school(), Some(&source_b.school.id));
    let before_head = super::authority_head(&store_b.store)?;

    let result = store_b
        .store
        .apply_roster_member_withdrawal_v1(prepared, &approval, &digest);
    check!(matches!(
        result,
        Err(census_store::StoreError::RevisionOriginMismatch { .. })
    ));
    check!(eq; super::authority_head(&store_b.store)?, before_head);
    check!(eq; super::physical(&store_b.store)?, before_physical);
    let (after, _) = store_b.assess()?;
    check!(eq; after.resolution(&source_b.athlete.id).school(), Some(&source_b.school.id));
    Ok(())
}
