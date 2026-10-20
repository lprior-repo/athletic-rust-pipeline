use super::super::{fixture, issuance, TestResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{ProfileDate, ProfileDateRangeV1, Sport};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{RosterMemberWithdrawalReviewV1, RosterReviewActorV1};
use serde_json::json;

const ORIGINAL_SOURCE: &str = "controlled-progress-original-source";
const WITHDRAWAL: &str = "controlled-progress-withdrawal";

#[test]
fn ordinary_progress_journal_does_not_invalidate_current_authority() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let source = fixture::install(&fixture, "22222", "77", Sport::OutdoorTrack, true)?;
    fixture.accept(&source.athlete.id)?;
    let original_approval = issuance::review_revision(
        &fixture,
        &source,
        ORIGINAL_SOURCE,
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let original_digest = super::super::super::hash(&original_approval);
    let before_physical = super::physical(&fixture.store)?;
    let before_head = super::authority_head(&fixture.store)?;
    let (accepted, _) = fixture.assess()?;
    check!(eq; accepted.resolution(&source.athlete.id).school(), Some(&source.school.id));

    let witness = super::source_witness(&fixture.store, ORIGINAL_SOURCE)?;
    let prepared = fixture.store.prepare_roster_member_withdrawal_v1(
        RosterAuthorityTextV1::parse(ORIGINAL_SOURCE)?,
        witness,
        RosterMemberWithdrawalReviewV1 {
            operation: RosterAuthorityTextV1::parse(WITHDRAWAL)?,
            actor: RosterReviewActorV1::LocalOperator,
            reviewed_at: fixture::instant()?,
            rationale: RosterAuthorityTextV1::parse(
                "Controlled withdrawal to test progress journal interaction",
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
    fixture.store.journal_done(
        "progress-phase",
        "progress-key",
        &json!({"role": "test_progress", "step": 1}),
    )?;
    check!(eq; super::authority_head(&fixture.store)?, before_head);
    let (still_accepted, _) = fixture.assess()?;
    check!(eq; still_accepted.resolution(&source.athlete.id).school(), Some(&source.school.id));
    let written = fixture
        .store
        .apply_roster_member_withdrawal_v1(prepared, &approval, &digest)?;
    check!(matches!(written, census_store::Application::Written(_)));

    let (withheld, _) = fixture.assess()?;
    check!(eq; withheld.resolution(&source.athlete.id).school(), None);
    let audit = withheld
        .audit(&source.athlete.id)
        .ok_or("withheld member audit")?;
    check!(eq; audit.unavailable().iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["reviewed_roster_member_withdrawn"]);
    let withdrawn_head = super::authority_head(&fixture.store)?;

    let replayed = fixture
        .store
        .replay_roster_source_review_v1(ORIGINAL_SOURCE, &original_digest)?;
    check!(matches!(replayed, census_store::Application::Repeated(_)));

    let (after_replay, _) = fixture.assess()?;
    check!(eq; after_replay.resolution(&source.athlete.id).school(), None);
    let after_replay_audit = after_replay
        .audit(&source.athlete.id)
        .ok_or("after-replay withdrawn member audit")?;
    check!(eq; after_replay_audit.unavailable().iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["reviewed_roster_member_withdrawn"]);

    check!(eq; super::physical(&fixture.store)?, before_physical);
    check!(eq; super::authority_head(&fixture.store)?, withdrawn_head);

    Ok(())
}
