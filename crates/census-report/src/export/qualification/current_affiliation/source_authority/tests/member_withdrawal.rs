use super::*;
use census_crawl::participation_evidence::roster_source_issuance::assess_reviewed_roster_source_v1;
use census_domain::model::{ProfileDate, ProfileDateRangeV1};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{RosterMemberWithdrawalReviewV1, RosterReviewActorV1};

mod cold_bundle;
mod fallback;
mod guards;
mod isolation;
mod progress;
mod replay;
mod scope;

#[test]
fn reviewed_withdrawal_withholds_the_sole_current_school_without_erasing_its_source() -> TestResult
{
    let fixture = fixture::Fixture::new()?;
    let source = fixture::install(&fixture, "22222", "77", Sport::OutdoorTrack, true)?;
    fixture.accept(&source.athlete.id)?;
    let original_approval = issuance::review_revision(
        &fixture,
        &source,
        "controlled-member-original-source",
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let original_digest = super::super::hash(&original_approval);
    let before_physical = physical(&fixture.store)?;
    let (before, _) = fixture.assess()?;
    check!(eq; before.resolution(&source.athlete.id).school(), Some(&source.school.id));
    let witness = source_witness(&fixture.store, "controlled-member-original-source")?;
    let prepared = fixture.store.prepare_roster_member_withdrawal_v1(
        RosterAuthorityTextV1::parse("controlled-member-original-source")?,
        witness,
        RosterMemberWithdrawalReviewV1 {
            operation: RosterAuthorityTextV1::parse("controlled-member-withdrawal")?,
            actor: RosterReviewActorV1::LocalOperator,
            reviewed_at: fixture::instant()?,
            rationale: RosterAuthorityTextV1::parse(
                "Controlled withdrawal of the sole current-school member assertion",
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
    let written = fixture
        .store
        .apply_roster_member_withdrawal_v1(prepared, &approval, &digest)?;
    check!(matches!(written, census_store::Application::Written(_)));
    let (after, _) = fixture.assess()?;
    check!(eq; after.resolution(&source.athlete.id).school(), None);
    let audit = after
        .audit(&source.athlete.id)
        .ok_or("withdrawn member audit")?;
    check!(eq; audit.unavailable().iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["reviewed_roster_member_withdrawn"]);
    check!(eq; physical(&fixture.store)?, before_physical);
    let replayed = fixture
        .store
        .replay_roster_source_review_v1("controlled-member-original-source", &original_digest)?;
    check!(matches!(replayed, census_store::Application::Repeated(_)));
    let (after_replay, _) = fixture.assess()?;
    check!(eq; after_replay.resolution(&source.athlete.id).school(), None);
    check!(eq; physical(&fixture.store)?, before_physical);
    Ok(())
}

pub(super) fn source_witness(store: &Store, operation: &str) -> TestResult<Vec<u8>> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let claims = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    let claim = claims
        .claims(&mut grant)?
        .iter()
        .find(|claim| claim.operation() == operation)
        .ok_or("original reviewed source claim")?;
    let assessed = assess_reviewed_roster_source_v1(claim, &mut grant)?;
    let packet = assessed.source(&mut grant)?.checked_packet(&mut grant)?;
    let roster = packet
        .roster_verdict()
        .roster()
        .ok_or("original parsed roster")?;
    let locator = roster
        .athletes
        .first()
        .ok_or("original source member")?
        .locator();
    Ok(assessed
        .source(&mut grant)?
        .member_semantic_witness_v1(locator, &mut grant)?)
}

fn physical(store: &Store) -> TestResult<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut rows = Vec::new();
    store
        .snapshot()
        .for_each_physical_observation_bounded::<CanonicalAthlete>(
            Table::Athletes,
            fixture::budget(),
            |_, key, raw, _| {
                rows.push((key.to_vec(), raw.to_vec()));
                Ok(())
            },
        )?;
    Ok(rows)
}

fn authority_head(store: &Store) -> TestResult<Vec<u8>> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    Ok(current.authority_head_bytes(&mut grant)?.to_vec())
}
