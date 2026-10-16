use super::*;
use census_domain::public_identity_history::local_membership_withholding::{
    LocalMembershipWithholdV1, MembershipAttachmentV1,
};
use census_domain::public_identity_history::{HistoryRequest, PublicResolution};
use sha2::{Digest, Sha256};

pub(super) fn withdraw(fixture: &Fixture, candidate: &AthleteId) -> TestResult {
    let before = fixture.store.snapshot().freeze_public_identity_v1()?;
    let accepted = before.replay()?;
    let record = accepted.history_records().iter().find(|record| {
        matches!(&record.request, HistoryRequest::BindCandidateMembership { membership, .. } if membership.candidate == candidate.cast())
    }).ok_or("membership attachment")?;
    let HistoryRequest::BindCandidateMembership {
        owner, membership, ..
    } = &record.request
    else {
        return Err("membership record required".into());
    };
    let inventory = fixture
        .writer
        .inspect_membership_withholding(&fixture.store, owner)?;
    let case = fixture
        .store
        .scan::<ReviewCase>(Table::ReviewCases)?
        .into_iter()
        .find(|case| case.member_ids.contains(&candidate.cast()))
        .ok_or("attachment review case")?;
    let request = LocalMembershipWithholdV1 {
        case_id: case.id,
        canonical: owner.clone(),
        attachment: MembershipAttachmentV1 {
            sequence: record.sequence,
            digest: record.digest.clone(),
            membership_id: membership.id.clone(),
        },
        expected_withheld_memberships: vec![membership.id.clone()],
        affected_record_keys: inventory.affected_record_keys().to_vec(),
        rationale: "Synthetic operator withdraws this exact transfer attachment".into(),
    };
    let prepared = fixture.writer.prepare_membership_withholding(
        &fixture.store,
        "withhold-transfer",
        request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    #[derive(serde::Serialize)]
    struct Approval<'a> {
        schema: &'static str,
        actor: &'static str,
        prepared: &'a census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdingRecordV1,
    }
    let schema = "local-source-membership-withholding-approval/v1";
    let bytes = serde_json::to_vec(&Approval {
        schema,
        actor: "local_operator",
        prepared: prepared.record(),
    })?;
    let mut hasher = Sha256::new();
    hasher.update(schema.as_bytes());
    hasher.update([0]);
    hasher.update(&bytes);
    let digest = format!("{:x}", hasher.finalize());
    let applied = fixture.writer.apply_membership_withholding(
        &fixture.store,
        prepared,
        bytes,
        digest.clone(),
    )?;
    check!(applied.introduced_public_ids().is_empty());
    let repeated = fixture
        .writer
        .replay_membership_withholding(&fixture.store, "withhold-transfer", &digest)?
        .ok_or("withholding replay")?;
    check!(repeated.replayed());
    check!(eq; repeated.head(), applied.head());
    let after = fixture
        .store
        .snapshot()
        .freeze_public_identity_v1()?
        .replay()?;
    check!(eq; after.public_ids().count(), 1);
    check!(eq; after.resolve_candidate(&candidate.cast())?, PublicResolution::Unresolved);
    check!(eq; after.resolve_public(owner)?, accepted.resolve_public(owner)?);
    check!(matches!(
        before.replay()?.resolve_candidate(&candidate.cast())?,
        PublicResolution::Canonical(_)
    ));
    Ok(())
}
