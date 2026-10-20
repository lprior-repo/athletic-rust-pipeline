use super::*;
use census_crawl::participation_evidence::roster_source_issuance::assess_reviewed_roster_source_v1;
use census_domain::model::{ProfileDate, ProfileDateRangeV1, SourceNamespace, SourceSubject};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{
    replay_frozen_roster_reviewed_claims_v1, RosterMemberWithdrawalReviewV1, RosterReviewActorV1,
};

pub(super) fn member_claim(
    claim: &census_store::ReviewedRosterSourceClaimV1,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<String> {
    let assessed = assess_reviewed_roster_source_v1(claim, grant)?;
    let packet = assessed.source(grant)?.checked_packet(grant)?;
    let roster = packet
        .roster_verdict()
        .roster()
        .ok_or("cold-bundle original parsed roster")?;
    let member = roster
        .athletes
        .first()
        .ok_or("cold-bundle original source member")?;
    let locator = member.locator();
    let person = SourceSubject::new(SourceNamespace::MilesplitAthlete, "77".to_string())?;
    let proposal = assessed.proposal(grant)?;
    Ok(census_domain::model::serialized_digest(&(
        proposal, &person, &locator,
    ))?)
}

#[test]
fn frozen_closure_replays_retain_source_history_without_live_store() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let source = fixture::install(&fixture, "22222", "77", Sport::OutdoorTrack, true)?;
    fixture.accept(&source.athlete.id)?;
    issuance::review_revision(
        &fixture,
        &source,
        "cold-bundle-original-source",
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let original_snapshot = fixture.assess()?;
    check!(eq;
        original_snapshot.0.resolution(&source.athlete.id).school(),
        Some(&source.school.id));

    let (g0_bytes, g0_reference) = {
        let snapshot = fixture.store.snapshot();
        let mut grant = AttachmentReplayGrantV1::new();
        let current = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
        let bundle = current.freeze_closure(&mut grant)?;
        (bundle.bytes().to_vec(), bundle.reference().clone())
    };

    let (g1_bytes, g1_reference) = {
        let witness = source_witness(&fixture.store, "cold-bundle-original-source")?;
        let prepared = fixture.store.prepare_roster_member_withdrawal_v1(
            RosterAuthorityTextV1::parse("cold-bundle-original-source")?,
            witness,
            RosterMemberWithdrawalReviewV1 {
                operation: RosterAuthorityTextV1::parse("cold-bundle-withdrawal")?,
                actor: RosterReviewActorV1::LocalOperator,
                reviewed_at: fixture::instant()?,
                rationale: RosterAuthorityTextV1::parse(
                    "Controlled withdrawal for frozen-closure replay test",
                )?,
                scope: ProfileDateRangeV1::new(
                    ProfileDate::parse("2026-10-16")?,
                    ProfileDate::parse("2026-10-16")?,
                )?,
            },
            AttachmentReplayGrantV1::new(),
        )?;
        let approval = prepared.approval_bytes().to_vec();
        let approval_digest = prepared.approval_digest().to_string();
        let written = fixture.store.apply_roster_member_withdrawal_v1(
            prepared,
            &approval,
            &approval_digest,
        )?;
        check!(matches!(written, census_store::Application::Written(_)));
        let snapshot = fixture.store.snapshot();
        let mut grant = AttachmentReplayGrantV1::new();
        let current = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
        let bundle = current.freeze_closure(&mut grant)?;
        (bundle.bytes().to_vec(), bundle.reference().clone())
    };

    drop(fixture);

    let g0_claim_id;
    {
        let mut grant = AttachmentReplayGrantV1::new();
        let replayed =
            replay_frozen_roster_reviewed_claims_v1(&g0_bytes, &g0_reference, &mut grant)?;
        let claims = replayed.claims(&mut grant)?;
        check!(eq; claims.len(), 1);
        let claim = claims
            .first()
            .ok_or("cold-bundle G0 original claim absent")?;
        check!(eq; claim.operation(), "cold-bundle-original-source");
        g0_claim_id = member_claim(claim, &mut grant)?;
        let withdrawals = replayed.member_withdrawals(&mut grant)?;
        check!(eq; withdrawals.len(), 0);
    }

    {
        let mut grant = AttachmentReplayGrantV1::new();
        let replayed =
            replay_frozen_roster_reviewed_claims_v1(&g1_bytes, &g1_reference, &mut grant)?;
        let claims = replayed.claims(&mut grant)?;
        check!(eq; claims.len(), 1);
        let claim = claims
            .first()
            .ok_or("cold-bundle G1 original claim absent")?;
        check!(eq; claim.operation(), "cold-bundle-original-source");
        let g1_claim_id = member_claim(claim, &mut grant)?;
        check!(eq; g0_claim_id, g1_claim_id);

        let withdrawals = replayed.member_withdrawals(&mut grant)?;
        check!(eq; withdrawals.len(), 1);
        let withdrawal = withdrawals
            .first()
            .ok_or("cold-bundle G1 withdrawal absent")?;
        check!(eq; withdrawal.operation(), "cold-bundle-withdrawal");
        let decision = withdrawal.decision();
        check!(eq; decision.source_operation(), "cold-bundle-original-source");
        check!(eq; decision.member_claim(), g1_claim_id);
        let person = decision.person();
        check!(eq; person.namespace(), &SourceNamespace::MilesplitAthlete);
        check!(eq; person.provider_id(), "77");
        let scope = decision.scope();
        check!(eq; scope.from(), &ProfileDate::parse("2026-10-16")?);
        check!(eq; scope.through(), &ProfileDate::parse("2026-10-16")?);
    }

    Ok(())
}

#[test]
fn frozen_withdrawal_refuses_selected_sequence_before_its_inspection() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let source = fixture::install(&fixture, "22222", "77", Sport::OutdoorTrack, true)?;
    fixture.accept(&source.athlete.id)?;
    issuance::review_revision(
        &fixture,
        &source,
        "known-time-original-source",
        issuance::CaptureRevision::RowLocalV3,
    )?;
    let earlier_sequence = fixture.store.snapshot().sequence();
    fixture.store.journal_done(
        "known-time-progress",
        "before-inspection",
        &serde_json::json!({"step": 1}),
    )?;
    let inspected_sequence = fixture.store.snapshot().sequence();
    check!(inspected_sequence > earlier_sequence);
    let witness = source_witness(&fixture.store, "known-time-original-source")?;
    let prepared = fixture.store.prepare_roster_member_withdrawal_v1(
        RosterAuthorityTextV1::parse("known-time-original-source")?,
        witness,
        RosterMemberWithdrawalReviewV1 {
            operation: RosterAuthorityTextV1::parse("known-time-withdrawal")?,
            actor: RosterReviewActorV1::LocalOperator,
            reviewed_at: fixture::instant()?,
            rationale: RosterAuthorityTextV1::parse("Controlled known-time refusal regression")?,
            scope: ProfileDateRangeV1::new(
                ProfileDate::parse("2026-10-16")?,
                ProfileDate::parse("2026-10-16")?,
            )?,
        },
        AttachmentReplayGrantV1::new(),
    )?;
    let approval = prepared.approval_bytes().to_vec();
    let approval_digest = prepared.approval_digest().to_string();
    let approved: serde_json::Value = serde_json::from_slice(&approval)?;
    check!(eq; approved.pointer("/reviewed_claims/inspected_sequence").and_then(serde_json::Value::as_u64), Some(inspected_sequence));
    fixture
        .store
        .apply_roster_member_withdrawal_v1(prepared, &approval, &approval_digest)?;
    let snapshot = fixture.store.snapshot();
    check!(snapshot.sequence() > inspected_sequence);
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    let bundle = current.freeze_closure(&mut grant)?;
    let mut replay_grant = AttachmentReplayGrantV1::new();
    let valid = replay_frozen_roster_reviewed_claims_v1(
        bundle.bytes(),
        bundle.reference(),
        &mut replay_grant,
    )?;
    check!(eq; valid.member_withdrawals(&mut replay_grant)?.len(), 1);
    let (bytes, reference) =
        relabel_selected_sequence(bundle.bytes(), bundle.reference(), earlier_sequence)?;
    let replayed = replay_frozen_roster_reviewed_claims_v1(
        &bytes,
        &reference,
        &mut AttachmentReplayGrantV1::new(),
    );
    check!(matches!(
        replayed,
        Err(census_store::StoreError::RosterSourceIssuance(
            census_store::RosterSourceIssuanceStoreError::Corrupt(
                "review inspection follows publication"
            )
        ))
    ));
    Ok(())
}

fn relabel_selected_sequence(
    bytes: &[u8],
    reference: &census_store::RosterFrozenReviewedClaimsReferenceV1,
    sequence: u64,
) -> TestResult<(Vec<u8>, census_store::RosterFrozenReviewedClaimsReferenceV1)> {
    let original = std::str::from_utf8(bytes)?;
    let marker = format!(
        "\"selected_snapshot_sequence\":{}",
        reference.selected_snapshot_sequence()
    );
    check!(eq; original.matches(&marker).count(), 1);
    let changed = original
        .replacen(
            &marker,
            &format!("\"selected_snapshot_sequence\":{sequence}"),
            1,
        )
        .into_bytes();
    let archive: serde_json::Value = serde_json::from_slice(&changed)?;
    check!(eq; archive.get("selected_snapshot_sequence").and_then(serde_json::Value::as_u64), Some(sequence));
    let mut expected = serde_json::to_value(reference)?;
    *expected
        .get_mut("selected_snapshot_sequence")
        .ok_or("reference sequence absent")? = sequence.into();
    *expected.get_mut("bytes").ok_or("reference size absent")? =
        u64::try_from(changed.len())?.into();
    *expected
        .get_mut("digest")
        .ok_or("reference digest absent")? = super::super::super::hash(&changed).into();
    Ok((changed, serde_json::from_value(expected)?))
}
