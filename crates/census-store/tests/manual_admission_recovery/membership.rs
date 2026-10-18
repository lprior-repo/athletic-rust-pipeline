use super::TestResult;
use census_domain::public_identity_history::HistoryRequest;
use census_store::{PublicIdentityWriterV1, SourceCandidateMembershipRequestV1, Store};

#[path = "../public_identity_crash/membership_evidence.rs"]
mod evidence;
pub(super) use evidence::install;

pub(super) fn withholding(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    attachment: &SourceCandidateMembershipRequestV1,
) -> TestResult<
    census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdV1,
> {
    use census_domain::public_identity_history::local_membership_withholding::{
        LocalMembershipWithholdV1, MembershipAttachmentV1,
    };
    writer.extend_source_candidate(
        store,
        "mixed-admission-attachment",
        attachment,
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("attached snapshot")?;
    let record = accepted
        .history_records()
        .last()
        .ok_or("attachment record")?;
    let HistoryRequest::BindCandidateMembership { membership, .. } = &record.request else {
        return Err("attachment kind".into());
    };
    let target = MembershipAttachmentV1 {
        sequence: record.sequence,
        digest: record.digest.clone(),
        membership_id: membership.id.clone(),
    };
    let inventory = writer.inspect_membership_withholding(
        store,
        "admission-withhold-inventory",
        &attachment.owner,
    )?;
    Ok(LocalMembershipWithholdV1 {
        case_id: attachment.case_id.clone(),
        canonical: attachment.owner.clone(),
        expected_withheld_memberships: writer.preview_membership_withholding(
            store,
            &attachment.owner,
            &target,
        )?,
        attachment: target,
        affected_record_keys: inventory.affected_record_keys().to_vec(),
        rationale: "Synthetic operator withholds this retained source attachment".into(),
    })
}
