use super::{HeldGraphWitness, Payload};
use crate::public_identity::{
    authority::graph,
    graph_reference::GraphAuthorityReferenceV1,
    paged_generation::VerifiedRawGraph,
    retained::{GraphRead, Rows},
    witness, PublicIdentityError,
};
use crate::StoreResult;
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV3;
use census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdingRecordV2;
use census_domain::public_identity_history::{
    prepare_next, HistoryProjection, HistoryRecord, HistoryRequest, PreparedHistoryRecord,
};
use std::collections::BTreeSet;

pub(in crate::public_identity) fn rebuild_manual(
    reader: &impl GraphRead,
    reference: &GraphAuthorityReferenceV1,
    record: &LocalIdentityAdjudicationRecordV3,
    previous: &HistoryProjection,
    records: &[HistoryRecord],
) -> StoreResult<PreparedHistoryRecord> {
    reference.require_domain(&record.graph)?;
    validate_binding(
        reference,
        &record.store,
        record.evidence_generation,
        record.derived_generation,
        record.snapshot_sequence,
    )?;
    if &record.expected_head != previous.head() {
        return Err(PublicIdentityError::HeadMoved.into());
    }
    let view = VerifiedRawGraph::verify(reader, reference.capture(), reference.seal())?;
    let members = graph::members(&view, previous, &record.request.canonical)?;
    if members != record.members {
        return Err(PublicIdentityError::Corrupt("graph manual members").into());
    }
    let mut case_members = graph::review_case(&view, &record.request.case_id)?.member_ids;
    case_members.sort();
    if case_members != members {
        return Err(PublicIdentityError::Corrupt("graph manual case members").into());
    }
    crate::public_identity::local_adjudication::prepare::validate_prior(&record.request, records)?;
    validate_superseded(reader, record)?;
    if graph::record_keys(&view, &members)? != record.request.affected_record_keys {
        return Err(PublicIdentityError::Corrupt("graph manual affected keys").into());
    }
    let claims = graph::membership_inventory(&view, &members, previous, records)?;
    let request = record
        .history_request(&claims)
        .map_err(PublicIdentityError::from)?;
    let prepared = prepare_next(
        &record.store,
        previous.head(),
        request,
        crate::public_identity::REPLAY_LIMITS,
    )
    .map_err(PublicIdentityError::from)?;
    previous
        .clone()
        .extend_candidate(&prepared.uncommitted_record)
        .map_err(PublicIdentityError::from)?;
    Ok(prepared)
}
pub(in crate::public_identity) fn rebuild_withholding(
    reader: &impl GraphRead,
    reference: &GraphAuthorityReferenceV1,
    record: &LocalMembershipWithholdingRecordV2,
    previous: &HistoryProjection,
    records: &[HistoryRecord],
) -> StoreResult<PreparedHistoryRecord> {
    reference.require_domain(&record.graph)?;
    validate_binding(
        reference,
        &record.store,
        record.evidence_generation,
        record.derived_generation,
        record.snapshot_sequence,
    )?;
    if &record.expected_head != previous.head() {
        return Err(PublicIdentityError::HeadMoved.into());
    }
    let view = VerifiedRawGraph::verify(reader, reference.capture(), reference.seal())?;
    let members = graph::members(&view, previous, &record.request.canonical)?;
    if members != record.members {
        return Err(PublicIdentityError::Corrupt("graph withholding members").into());
    }
    validate_attachment(reader, record, records)?;
    if graph::record_keys(&view, &members)? != record.request.affected_record_keys {
        return Err(PublicIdentityError::Corrupt("graph withholding affected keys").into());
    }
    let claims = graph::membership_inventory(&view, &members, previous, records)?;
    let request = record
        .history_request(&claims)
        .map_err(PublicIdentityError::from)?;
    let history = prepare_next(
        &record.store,
        previous.head(),
        request,
        crate::public_identity::REPLAY_LIMITS,
    )
    .map_err(PublicIdentityError::from)?;
    let after = previous
        .clone()
        .extend_candidate(&history.uncommitted_record)
        .map_err(PublicIdentityError::from)?;
    if !history.established_ids.is_empty()
        || after.withheld_memberships(history.next_head.sequence)
            != Some(record.request.expected_withheld_memberships.as_slice())
    {
        return Err(PublicIdentityError::Corrupt("graph withholding computed member set").into());
    }
    Ok(history)
}
fn validate_binding(
    reference: &GraphAuthorityReferenceV1,
    identity: &StoreIdentity,
    evidence: u64,
    derived: u64,
    sequence: u64,
) -> StoreResult<()> {
    let capture = reference.capture();
    if capture.store_identity() != identity
        || capture.evidence_generation() != evidence
        || capture.derived_generation() != derived
        || capture.snapshot_sequence() != sequence
    {
        return Err(PublicIdentityError::Corrupt("manual complete capture binding").into());
    }
    Ok(())
}
fn validate_superseded(
    reader: &impl GraphRead,
    record: &LocalIdentityAdjudicationRecordV3,
) -> StoreResult<()> {
    record.request.supersedes.iter().try_for_each(|prior| {
        if prior.sequence > record.expected_head.sequence {
            return Err(
                PublicIdentityError::Corrupt("manual superseded operation position").into(),
            );
        }
        let held = load(reader, prior.sequence)?;
        let case = match &held.payload {
            Payload::Reviewed { case_id, .. } => case_id,
            Payload::Membership { request, .. } => &request.case_id,
            _ => {
                return Err(
                    PublicIdentityError::Corrupt("manual superseded graph authority kind").into(),
                )
            }
        };
        if case != &record.request.case_id || held.record_digest != prior.digest {
            return Err(
                PublicIdentityError::Corrupt("manual superseded graph case binding").into(),
            );
        }
        Ok(())
    })
}
fn validate_attachment(
    reader: &impl GraphRead,
    record: &LocalMembershipWithholdingRecordV2,
    records: &[HistoryRecord],
) -> StoreResult<()> {
    let prior = &record.request.attachment;
    if prior.sequence > record.expected_head.sequence {
        return Err(PublicIdentityError::Corrupt("withholding future attachment").into());
    }
    let original = records
        .iter()
        .find(|entry| entry.sequence == prior.sequence)
        .ok_or(PublicIdentityError::Corrupt(
            "withholding attachment missing",
        ))?;
    let HistoryRequest::BindCandidateMembership { membership, .. } = &original.request else {
        return Err(PublicIdentityError::Corrupt("withholding attachment kind").into());
    };
    if original.digest != prior.digest || membership.id != prior.membership_id {
        return Err(PublicIdentityError::Corrupt("withholding attachment reference").into());
    }
    let held = load(reader, prior.sequence)?;
    let Payload::Membership { request, .. } = &held.payload else {
        return Err(PublicIdentityError::Corrupt("withholding graph attachment kind").into());
    };
    if held.record_digest != prior.digest || request.case_id != record.request.case_id {
        return Err(PublicIdentityError::Corrupt("withholding graph attachment case").into());
    }
    Ok(())
}
fn load(reader: &impl GraphRead, sequence: u64) -> StoreResult<HeldGraphWitness> {
    let key = witness::key(sequence);
    let bytes = reader
        .row(Rows::Witness, &key)?
        .ok_or(PublicIdentityError::Corrupt(
            "missing graph authority witness",
        ))?;
    HeldGraphWitness::decode(&key, &bytes)
}
impl HeldGraphWitness {
    pub(in crate::public_identity) fn require_available(
        &self,
        reader: &impl GraphRead,
        identity: &StoreIdentity,
        superseded: &BTreeSet<String>,
    ) -> StoreResult<()> {
        if self.graph.capture().store_identity() != identity {
            return Err(PublicIdentityError::ForeignStore.into());
        }
        if let Some(material) = self.reviewed_material(reader)? {
            if superseded.contains(&material) {
                return Err(PublicIdentityError::Corrupt(
                    "graph authority superseded by local adjudication",
                )
                .into());
            }
        }
        Ok(())
    }
    pub(in crate::public_identity) fn retain_superseded_material(
        &self,
        reader: &impl GraphRead,
        superseded: &mut BTreeSet<String>,
    ) -> StoreResult<()> {
        let material = match &self.payload {
            Payload::Manual { adjudication, .. } => {
                let view =
                    VerifiedRawGraph::verify(reader, self.graph.capture(), self.graph.seal())?;
                Some(graph::identity_material(&view, &adjudication.members)?)
            }
            Payload::Withholding { adjudication, .. } => {
                let held = load(reader, adjudication.request.attachment.sequence)?;
                if !matches!(held.payload, Payload::Membership { .. })
                    || held.record_digest != adjudication.request.attachment.digest
                {
                    return Err(
                        PublicIdentityError::Corrupt("withheld graph material binding").into(),
                    );
                }
                Some(
                    held.reviewed_material(reader)?
                        .ok_or(PublicIdentityError::Corrupt(
                            "withheld graph material missing",
                        ))?,
                )
            }
            _ => None,
        };
        if let Some(material) = material {
            superseded.insert(material);
        }
        Ok(())
    }
    fn reviewed_material(&self, reader: &impl GraphRead) -> StoreResult<Option<String>> {
        let case_id = match &self.payload {
            Payload::Reviewed { case_id, .. } => case_id,
            Payload::Membership { request, .. } => &request.case_id,
            _ => return Ok(None),
        };
        let view = VerifiedRawGraph::verify(reader, self.graph.capture(), self.graph.seal())?;
        let case = graph::review_case(&view, case_id)?;
        graph::identity_material(&view, &case.member_ids).map(Some)
    }
}
