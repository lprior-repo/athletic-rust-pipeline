use census_domain::model::{ReviewPacket, VerdictBatch};
use serde::{Deserialize, Serialize};

use crate::ask::Answer;
use crate::ModelClient;

mod writer;

#[derive(Serialize, Deserialize)]
pub(crate) struct Audit {
    pub(crate) policy: String,
    pub(crate) packet: Option<ReviewPacket>,
    pub(crate) evidence_digest: String,
    pub(crate) lanes: [LaneAudit; 2],
    pub(crate) outcome: String,
}

impl Audit {
    pub(crate) fn encode(&self) -> census_store::StoreResult<String> {
        writer::encode(self)
    }

    pub(crate) fn replay(
        row: &census_domain::model::ReviewVerdictRecord,
        packet: &ReviewPacket,
    ) -> Option<Self> {
        if row.reviewer != "dual-independent-consensus"
            || row.rationale.len() > writer::MAX_AUDIT_BYTES
        {
            return None;
        }
        let audit: Self = serde_json::from_str(&row.rationale).ok()?;
        (audit.policy == super::POLICY
            && audit.packet.as_ref() == Some(packet)
            && census_domain::model::serialized_digest(&audit.packet)
                .is_ok_and(|digest| digest == audit.evidence_digest))
        .then_some(audit)
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct LaneAudit {
    endpoint: String,
    model: String,
    timeout_ms: u128,
    max_tokens: u32,
    response_format: crate::ModelResponseFormat,
    request_digest: Option<String>,
    pub(crate) status: String,
    error: Option<String>,
    batch: Option<VerdictBatch>,
    adjudications: Vec<String>,
    dropped: usize,
}

impl LaneAudit {
    pub(crate) fn binding(client: &ModelClient, request_digest: Option<String>) -> Self {
        let options = client.options();
        Self {
            endpoint: options.completions_url(),
            model: options.model_name().to_string(),
            timeout_ms: options.timeout().as_millis(),
            max_tokens: options.max_tokens(),
            response_format: options.response_format(),
            request_digest,
            status: "missing_subject".to_string(),
            error: None,
            batch: None,
            adjudications: Vec::new(),
            dropped: 0,
        }
    }

    pub(crate) fn same_binding(&self, other: &Self) -> bool {
        self.request_digest.is_some()
            && self.endpoint == other.endpoint
            && self.model == other.model
            && self.timeout_ms == other.timeout_ms
            && self.max_tokens == other.max_tokens
            && self.response_format == other.response_format
            && self.request_digest == other.request_digest
    }

    pub(crate) fn replay_answer(
        &self,
        packet: &ReviewPacket,
        family: crate::ReviewFamily,
    ) -> Option<Answer> {
        if self.status != "answered" || self.error.is_some() || self.dropped != 0 {
            return None;
        }
        let batch = self.batch.as_ref()?;
        let (verdicts, dropped) = crate::triage(packet, family, batch.clone());
        if dropped != self.dropped
            || verdicts.len() != self.adjudications.len()
            || !verdicts
                .iter()
                .zip(&self.adjudications)
                .all(|((_, adjudication), retained)| format!("{adjudication:?}") == *retained)
        {
            return None;
        }
        let answer = Answer::Answered {
            batch: batch.clone(),
            verdicts,
            dropped,
        };
        super::usable(&answer).is_some().then_some(answer)
    }

    pub(crate) fn retain(mut self, answer: Answer) -> Self {
        match answer {
            Answer::Failed(error) => {
                self.status = "failed".to_string();
                self.error = Some(error.to_string());
            }
            Answer::Answered {
                batch,
                verdicts,
                dropped,
            } => {
                self.status = "answered".to_string();
                self.batch = Some(batch);
                self.adjudications = verdicts
                    .into_iter()
                    .map(|(_, adjudication)| format!("{adjudication:?}"))
                    .collect();
                self.dropped = dropped;
            }
        }
        self
    }
}
