use census_domain::model::{ReviewPacket, ReviewVerdict, VerdictBatch};

use super::model::{ModelClient, ModelError};
use super::verdicts::{triage, Adjudication};
use super::ReviewFamily;

pub(super) enum Answer {
    Failed(ModelError),
    Answered {
        batch: VerdictBatch,
        verdicts: Vec<(ReviewVerdict, Adjudication)>,
        dropped: usize,
    },
}

pub(super) async fn ask_case(
    client: &ModelClient,
    packet: &ReviewPacket,
    family: ReviewFamily,
) -> Answer {
    let batch = match client.adjudicate(packet).await {
        Ok(batch) => batch,
        Err(error) => return Answer::Failed(error),
    };
    let (verdicts, dropped) = triage(packet, family, batch.clone());
    Answer::Answered {
        batch,
        verdicts,
        dropped,
    }
}
