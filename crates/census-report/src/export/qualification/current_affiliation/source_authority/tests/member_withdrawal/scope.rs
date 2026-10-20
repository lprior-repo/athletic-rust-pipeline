use super::super::{fixture, issuance, TestResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{ProfileDate, ProfileDateRangeV1, Sport};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{RosterMemberWithdrawalReviewV1, RosterReviewActorV1};

const CAPTURE_DAY: &str = "2026-10-16";
const ORIGINAL: &str = "controlled-scope-original-source";
const ENDING_BEFORE: &str = "controlled-scope-ending-before-capture-day";
const STARTING_AFTER: &str = "controlled-scope-starting-after-capture-day";
const WIDER_THAN_DAY: &str = "controlled-scope-wider-than-capture-day";
const EXACT_DAY: &str = "controlled-scope-exact-capture-day";

fn withdrawal_review(
    operation: &str,
    rationale: &str,
    scope: ProfileDateRangeV1,
) -> TestResult<RosterMemberWithdrawalReviewV1> {
    Ok(RosterMemberWithdrawalReviewV1 {
        operation: RosterAuthorityTextV1::parse(operation)?,
        actor: RosterReviewActorV1::LocalOperator,
        reviewed_at: fixture::instant()?,
        rationale: RosterAuthorityTextV1::parse(rationale)?,
        scope,
    })
}

fn rejected_withdrawal(
    fixture: &fixture::Fixture,
    source_operation: &str,
    operation: &str,
    rationale: &str,
    scope: ProfileDateRangeV1,
) -> TestResult {
    let witness = super::source_witness(&fixture.store, source_operation)?;
    let prepared = fixture.store.prepare_roster_member_withdrawal_v1(
        RosterAuthorityTextV1::parse(source_operation)?,
        witness,
        withdrawal_review(operation, rationale, scope)?,
        AttachmentReplayGrantV1::new(),
    );
    check!(matches!(
        prepared,
        Err(census_store::StoreError::Refused { .. })
    ));
    Ok(())
}

fn approved_withdrawal(
    fixture: &fixture::Fixture,
    source_operation: &str,
    operation: &str,
    rationale: &str,
    scope: ProfileDateRangeV1,
) -> TestResult {
    let witness = super::source_witness(&fixture.store, source_operation)?;
    let prepared = fixture.store.prepare_roster_member_withdrawal_v1(
        RosterAuthorityTextV1::parse(source_operation)?,
        witness,
        withdrawal_review(operation, rationale, scope)?,
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
    Ok(())
}

#[test]
fn withdrawal_scope_must_equal_the_original_capture_day() -> TestResult {
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
    let original_head = super::authority_head(&fixture.store)?;
    let (accepted, _) = fixture.assess()?;
    check!(eq; accepted.resolution(&source.athlete.id).school(), Some(&source.school.id));
    check!(accepted
        .audit(&source.athlete.id)
        .is_none_or(|audit| audit.unavailable().is_empty()));

    let rejected = [
        (
            ENDING_BEFORE,
            "Reviewed withdrawal scope ending before the original capture day",
            ProfileDateRangeV1::new(
                ProfileDate::parse("2026-10-01")?,
                ProfileDate::parse("2026-10-15")?,
            )?,
        ),
        (
            STARTING_AFTER,
            "Reviewed withdrawal scope starting after the original capture day",
            ProfileDateRangeV1::new(
                ProfileDate::parse("2026-10-17")?,
                ProfileDate::parse("2026-10-31")?,
            )?,
        ),
        (
            WIDER_THAN_DAY,
            "Reviewed withdrawal scope wider than the original capture day",
            ProfileDateRangeV1::new(
                ProfileDate::parse("2026-10-01")?,
                ProfileDate::parse("2027-06-30")?,
            )?,
        ),
    ];
    for (operation, rationale, scope) in rejected {
        rejected_withdrawal(&fixture, ORIGINAL, operation, rationale, scope)?;
        check!(eq; super::authority_head(&fixture.store)?, original_head,
            "rejected withdrawal {operation} appended an authority record");
        check!(eq; super::physical(&fixture.store)?, original_physical,
            "rejected withdrawal {operation} altered physical rows");
        let (current, _) = fixture.assess()?;
        check!(eq; current.resolution(&source.athlete.id).school(),
            Some(&source.school.id),
            "rejected withdrawal {operation} changed the selected school");
        check!(
            current
                .audit(&source.athlete.id)
                .is_none_or(|audit| audit.unavailable().is_empty()),
            "rejected withdrawal {operation} withheld the member"
        );
    }

    approved_withdrawal(
        &fixture,
        ORIGINAL,
        EXACT_DAY,
        "Reviewed withdrawal scope equal to the original capture day",
        ProfileDateRangeV1::new(
            ProfileDate::parse(CAPTURE_DAY)?,
            ProfileDate::parse(CAPTURE_DAY)?,
        )?,
    )?;
    let withdrawn_head = super::authority_head(&fixture.store)?;
    check!(ne; withdrawn_head, original_head);
    let (withheld, _) = fixture.assess()?;
    check!(eq; withheld.resolution(&source.athlete.id).school(), None,
        "the exact capture-day withdrawal must withhold the member");
    let audit = withheld
        .audit(&source.athlete.id)
        .ok_or("withheld member audit")?;
    check!(eq; audit.unavailable().iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["reviewed_roster_member_withdrawn"]);
    check!(eq; super::physical(&fixture.store)?, original_physical);

    let replayed = fixture
        .store
        .replay_roster_source_review_v1(ORIGINAL, &original_digest)?;
    check!(matches!(replayed, census_store::Application::Repeated(_)));
    check!(eq; super::authority_head(&fixture.store)?, withdrawn_head);
    let (replayed_view, _) = fixture.assess()?;
    check!(eq; replayed_view.resolution(&source.athlete.id).school(), None,
        "replaying the original positive command must not reactivate the withdrawal");
    let replayed_audit = replayed_view
        .audit(&source.athlete.id)
        .ok_or("replayed withdrawn member audit")?;
    check!(eq; replayed_audit.unavailable().iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["reviewed_roster_member_withdrawn"]);
    check!(eq; super::physical(&fixture.store)?, original_physical);
    Ok(())
}
