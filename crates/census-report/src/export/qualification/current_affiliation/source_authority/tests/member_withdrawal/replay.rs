use super::super::{fixture, issuance, TestResult};
use super::*;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{ProfileDate, ProfileDateRangeV1};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{RosterMemberWithdrawalReviewV1, RosterReviewActorV1};

const CUTOFF: &str = "2026-10-16";
const ORIGINAL: &str = "controlled-replay-original-source";
const NEGATIVE: &str = "controlled-replay-withdrawal";

fn reviewed_withdrawal(
    fixture: &fixture::Fixture,
    source_operation: &str,
    operation: &str,
    rationale: &str,
    scope: ProfileDateRangeV1,
) -> TestResult<String> {
    let witness = super::source_witness(&fixture.store, source_operation)?;
    let prepared = fixture.store.prepare_roster_member_withdrawal_v1(
        RosterAuthorityTextV1::parse(source_operation)?,
        witness,
        RosterMemberWithdrawalReviewV1 {
            operation: RosterAuthorityTextV1::parse(operation)?,
            actor: RosterReviewActorV1::LocalOperator,
            reviewed_at: fixture::instant()?,
            rationale: RosterAuthorityTextV1::parse(rationale)?,
            scope,
        },
        AttachmentReplayGrantV1::new(),
    )?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_string();
    let written = fixture
        .store
        .apply_roster_member_withdrawal_v1(prepared, &approval, &digest)?;
    check!(
        matches!(written, census_store::Application::Written(_)),
        "reviewed withdrawal {operation} was not applied"
    );
    Ok(digest)
}

#[test]
fn replayed_negative_receipt_preserves_withheld_authority_without_reviving_original() -> TestResult
{
    let fixture = fixture::Fixture::new()?;
    let source = fixture::install(&fixture, "22222", "77", Sport::OutdoorTrack, true)?;
    fixture.accept(&source.athlete.id)?;
    let original_approval = issuance::review_revision(
        &fixture,
        &source,
        ORIGINAL,
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let original_digest = super::super::super::hash(&original_approval);
    let original_physical = super::physical(&fixture.store)?;
    let (accepted, _) = fixture.assess()?;
    check!(eq; accepted.resolution(&source.athlete.id).school(), Some(&source.school.id));

    let negative_digest = reviewed_withdrawal(
        &fixture,
        ORIGINAL,
        NEGATIVE,
        "In-scope reviewed withdrawal covering the inclusive 2026-10-16 cutoff",
        ProfileDateRangeV1::new(ProfileDate::parse(CUTOFF)?, ProfileDate::parse(CUTOFF)?)?,
    )?;
    let (withheld, _) = fixture.assess()?;
    check!(eq; withheld.resolution(&source.athlete.id).school(), None);
    let audit = withheld
        .audit(&source.athlete.id)
        .ok_or("withdrawn member audit")?;
    check!(eq; audit.unavailable().iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["reviewed_roster_member_withdrawn"]);

    let head_before_negative_replay = super::authority_head(&fixture.store)?;
    let replayed_negative = fixture
        .store
        .replay_roster_source_review_v1(NEGATIVE, &negative_digest)?;
    check!(matches!(
        replayed_negative,
        census_store::Application::Repeated(_)
    ));
    let head_after_negative_replay = super::authority_head(&fixture.store)?;
    check!(eq; head_after_negative_replay, head_before_negative_replay);
    check!(eq; super::physical(&fixture.store)?, original_physical);
    let (still_withheld, _) = fixture.assess()?;
    check!(eq; still_withheld.resolution(&source.athlete.id).school(), None);
    let still_audit = still_withheld
        .audit(&source.athlete.id)
        .ok_or("replayed withdrawn member audit")?;
    check!(eq; still_audit.unavailable().iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["reviewed_roster_member_withdrawn"]);

    let replayed_positive = fixture
        .store
        .replay_roster_source_review_v1(ORIGINAL, &original_digest)?;
    check!(matches!(
        replayed_positive,
        census_store::Application::Repeated(_)
    ));
    check!(eq; super::authority_head(&fixture.store)?, head_after_negative_replay);
    check!(eq; super::physical(&fixture.store)?, original_physical);
    let (still_withheld_after_positive, _) = fixture.assess()?;
    check!(eq; still_withheld_after_positive.resolution(&source.athlete.id).school(), None);
    let final_audit = still_withheld_after_positive
        .audit(&source.athlete.id)
        .ok_or("final withdrawn member audit")?;
    check!(eq; final_audit.unavailable().iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["reviewed_roster_member_withdrawn"]);

    let wrong_negative_digest = "0000000000000000000000000000000000000000000000000000000000000000";
    let result = fixture
        .store
        .replay_roster_source_review_v1(NEGATIVE, wrong_negative_digest);
    check!(matches!(
        result,
        Err(census_store::StoreError::RosterSourceIssuance(
            census_store::RosterSourceIssuanceStoreError::OperationConflict
        ))
    ));
    check!(eq; super::authority_head(&fixture.store)?, head_after_negative_replay);
    check!(eq; super::physical(&fixture.store)?, original_physical);
    Ok(())
}
