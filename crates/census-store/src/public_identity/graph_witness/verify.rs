mod rebuild;
use super::{HeldGraphWitness, Payload};
use crate::public_identity::{
    authority::graph,
    codec, commit, correction, membership,
    paged_generation::VerifiedRawGraph,
    retained::{GraphRead, Revision, Rows},
    witness, PublicIdentityError,
};
use crate::StoreResult;
use census_domain::model::{serialized_digest, AppliedAthleteIdentity};
use census_domain::public_identity_history::{
    prepare_next, CandidateDecisionReference, HistoryProjection, HistoryRecord, HistoryRequest,
    PreparedHistoryRecord, PublicAthleteId, ReplayLimits,
};

impl HeldGraphWitness {
    pub(in crate::public_identity) fn verify(
        &self,
        reader: &impl GraphRead,
        previous: &HistoryProjection,
        prepared: &PreparedHistoryRecord,
        records: &[HistoryRecord],
    ) -> StoreResult<()> {
        self.validate_shape()?;
        self.graph.capture().require_identity_graph_v3_format()?;
        let record = &prepared.uncommitted_record;
        if self.record_digest != record.digest
            || previous.head() != &record.previous
            || self.graph.capture().store_identity() != &record.installation
        {
            return Err(PublicIdentityError::Corrupt("graph witness history binding").into());
        }
        reader.revision()?.includes(Revision {
            evidence_generation: self.graph.capture().evidence_generation(),
            derived_generation: self.graph.capture().derived_generation(),
            snapshot_sequence: self.graph.capture().snapshot_sequence(),
        })?;
        self.verify_receipt(reader)?;
        let view = VerifiedRawGraph::verify(reader, self.graph.capture(), self.graph.seal())?;
        let rebuilt = match &self.payload {
            Payload::Source { .. } => self.rebuild_source(reader, &view, previous, record)?,
            Payload::Reviewed { .. } => self.rebuild_reviewed(reader, &view, previous, record)?,
            Payload::Membership { .. } => {
                self.rebuild_membership(reader, &view, previous, record)?
            }
            Payload::Manual { adjudication, .. } => {
                super::manual::rebuild_manual(reader, &self.graph, adjudication, previous, records)?
            }
            Payload::Withholding { adjudication, .. } => super::manual::rebuild_withholding(
                reader,
                &self.graph,
                adjudication,
                previous,
                records,
            )?,
        };
        if &rebuilt != prepared {
            return Err(
                PublicIdentityError::Corrupt("graph authority prepared history binding").into(),
            );
        }
        previous
            .clone()
            .extend_candidate(record)
            .map_err(PublicIdentityError::from)?;
        self.verify_approval()?;
        Ok(())
    }
    fn verify_approval(&self) -> StoreResult<()> {
        let expected = match &self.payload {
            Payload::Manual { adjudication, .. } => {
                if self.operation != adjudication.operation
                    || self.observed_at != adjudication.observed_at
                {
                    return Err(PublicIdentityError::Corrupt("graph manual actor binding").into());
                }
                Some(
                    crate::public_identity::local_adjudication::consume::graph_approval_digest(
                        &crate::public_identity::local_adjudication::consume::graph_approval_bytes(
                            adjudication,
                            &self.graph,
                        )?,
                    ),
                )
            }
            Payload::Withholding { adjudication, .. } => {
                if self.operation != adjudication.operation
                    || self.observed_at != adjudication.observed_at
                {
                    return Err(
                        PublicIdentityError::Corrupt("graph withholding actor binding").into(),
                    );
                }
                Some(crate::public_identity::membership_withholding::consume::graph_approval_digest(&crate::public_identity::membership_withholding::consume::graph_approval_bytes(adjudication, &self.graph)?))
            }
            _ => None,
        };
        if expected.is_some_and(|expected| expected != self.request_digest) {
            return Err(PublicIdentityError::Corrupt("graph approval binding").into());
        }
        Ok(())
    }
    fn verify_receipt(&self, reader: &impl GraphRead) -> StoreResult<()> {
        let key = witness::receipt_key(&self.operation);
        let raw = reader
            .row(Rows::Receipt, &key)?
            .ok_or(PublicIdentityError::Corrupt("missing graph receipt"))?;
        let receipt: crate::Receipt = codec::decode(&key, &raw)?;
        if receipt.operation != key
            || receipt.digest != serialized_digest(self)?
            || receipt.at != self.observed_at
            || receipt.appended != 1
            || receipt.credited
        {
            return Err(PublicIdentityError::Corrupt("graph receipt binding").into());
        }
        Ok(())
    }
    fn verify_decision(
        &self,
        reader: &impl GraphRead,
        expected: &super::Decision,
        accepted: &AppliedAthleteIdentity,
    ) -> StoreResult<()> {
        let raw = reader
            .row(Rows::Decision, &expected.id)?
            .ok_or(PublicIdentityError::Corrupt("missing graph decision"))?;
        let decision: AppliedAthleteIdentity = codec::decode(&expected.id, &raw)?;
        if &decision != accepted
            || decision.id != expected.id
            || serialized_digest(&decision)? != expected.digest
            || decision.observed_at != self.observed_at
        {
            return Err(PublicIdentityError::Corrupt("graph accepted decision binding").into());
        }
        Ok(())
    }
}
