use super::super::{fixture, issuance, TestResult};
use super::cold_bundle::member_claim;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{ProfileDate, ProfileDateRangeV1};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{RosterMemberWithdrawalReviewV1, RosterReviewActorV1};

const CUTOFF: &str = "2026-10-16";
const OUTDOOR_SOURCE: &str = "controlled-fallback-outdoor-source";
const XC_SOURCE: &str = "controlled-fallback-xc-source";
const OUTDOOR_WITHDRAWAL: &str = "controlled-fallback-outdoor-withdrawal";

fn reviewed_withdrawal(
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
    Ok(())
}

fn selected_claims<'a>(
    view: &'a crate::export::qualification::VerifiedCurrentAffiliations,
    athlete: &census_domain::model::AthleteId,
) -> TestResult<&'a [String]> {
    match view.resolution(athlete) {
        crate::export::qualification::CurrentAffiliationResolution::Resolved {
            evidence_refs,
            ..
        } => Ok(evidence_refs),
        _ => Err("remaining clean claim did not resolve the school".into()),
    }
}

#[test]
fn withdrawing_one_claim_keeps_affiliation_via_remaining_claims() -> TestResult {
    let fixture = fixture::Fixture::new()?;

    let (outdoor, xc) = fixture::install_dual_programs(&fixture, "22222", "77")?;
    fixture.accept(&outdoor.athlete.id)?;
    let outdoor_approval = issuance::review_revision(
        &fixture,
        &outdoor,
        OUTDOOR_SOURCE,
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let outdoor_digest = super::super::super::hash(&outdoor_approval);

    let xc_approval = issuance::review_revision(
        &fixture,
        &xc,
        XC_SOURCE,
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let xc_digest = super::super::super::hash(&xc_approval);

    let original_physical = super::physical(&fixture.store)?;
    let (both, _) = fixture.assess()?;
    check!(eq; both.resolution(&outdoor.athlete.id).school(), Some(&outdoor.school.id));
    let both_audit = both.audit(&outdoor.athlete.id).ok_or("both-claims audit")?;
    check!(both_audit.unavailable().is_empty());

    let (outdoor_claim_id, xc_claim_id) = {
        let snapshot = fixture.store.snapshot();
        let mut grant = AttachmentReplayGrantV1::new();
        let current = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
        let claims = current.claims(&mut grant)?;
        let outdoor_claim = claims
            .iter()
            .find(|c| c.operation() == OUTDOOR_SOURCE)
            .ok_or("outdoor claim absent")?;
        let xc_claim = claims
            .iter()
            .find(|c| c.operation() == XC_SOURCE)
            .ok_or("xc claim absent")?;
        let outdoor_claim_id = member_claim(outdoor_claim, &mut grant)?;
        let xc_claim_id = member_claim(xc_claim, &mut grant)?;
        (outdoor_claim_id, xc_claim_id)
    };
    let mut expected = vec![outdoor_claim_id.clone(), xc_claim_id.clone()];
    expected.sort();
    check!(eq; selected_claims(&both, &outdoor.athlete.id)?, expected.as_slice());

    reviewed_withdrawal(
        &fixture,
        OUTDOOR_SOURCE,
        OUTDOOR_WITHDRAWAL,
        "Controlled withdrawal of OutdoorTrack claim; CrossCountry claim should sustain affiliation",
        ProfileDateRangeV1::new(
            ProfileDate::parse(CUTOFF)?,
            ProfileDate::parse(CUTOFF)?,
        )?,
    )?;

    let (after_withdrawal, _) = fixture.assess()?;
    check!(eq; after_withdrawal
        .resolution(&outdoor.athlete.id)
        .school(), Some(&outdoor.school.id));
    let after_audit = after_withdrawal
        .audit(&outdoor.athlete.id)
        .ok_or("after-withdrawal audit")?;
    check!(eq; selected_claims(&after_withdrawal, &outdoor.athlete.id)?, std::slice::from_ref(&xc_claim_id));
    check!(after_audit.references().contains(&outdoor_claim_id));
    let unavailable: Vec<&str> = after_audit
        .unavailable()
        .iter()
        .map(String::as_str)
        .collect();
    check!(unavailable.contains(&"reviewed_roster_member_withdrawn"));
    let head_before_replay = super::authority_head(&fixture.store)?;

    let replayed_outdoor = fixture
        .store
        .replay_roster_source_review_v1(OUTDOOR_SOURCE, &outdoor_digest)?;
    check!(matches!(
        replayed_outdoor,
        census_store::Application::Repeated(_)
    ));
    let replayed_xc = fixture
        .store
        .replay_roster_source_review_v1(XC_SOURCE, &xc_digest)?;
    check!(matches!(
        replayed_xc,
        census_store::Application::Repeated(_)
    ));

    check!(eq; super::authority_head(&fixture.store)?, head_before_replay);
    check!(eq; super::physical(&fixture.store)?, original_physical);

    let (after_replay, _) = fixture.assess()?;
    check!(eq; after_replay
        .resolution(&outdoor.athlete.id)
        .school(), Some(&outdoor.school.id));
    let after_replay_audit = after_replay
        .audit(&outdoor.athlete.id)
        .ok_or("after-replay audit")?;
    check!(eq; selected_claims(&after_replay, &outdoor.athlete.id)?, std::slice::from_ref(&xc_claim_id));
    let after_replay_unavailable: Vec<&str> = after_replay_audit
        .unavailable()
        .iter()
        .map(String::as_str)
        .collect();
    check!(after_replay_unavailable.contains(&"reviewed_roster_member_withdrawn"));
    check!(after_replay_audit.references().contains(&outdoor_claim_id));

    Ok(())
}
