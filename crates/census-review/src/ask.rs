
use census_domain::model::{ReviewCase, ReviewVerdict};

use super::model::{ModelClient, ModelError};
use super::packets::SubjectIndex;
use super::verdicts::{triage, Adjudication};
use super::ReviewFamily;

pub(super) enum Answer {
    NoSubject,
    Failed(ModelError),
    Answered {
        verdicts: Vec<(ReviewVerdict, Adjudication)>,
        dropped: usize,
    },
}

pub(super) async fn ask_case(
    client: &ModelClient,
    subjects: &SubjectIndex,
    case: &ReviewCase,
    family: ReviewFamily,
) -> Answer {
    let Some(packet) = subjects.packet(case, family) else {
        return Answer::NoSubject;
    };
    let batch = match client.adjudicate(&packet).await {
        Ok(batch) => batch,
        Err(error) => return Answer::Failed(error),
    };
    let (verdicts, dropped) = triage(&packet, family, batch);
    Answer::Answered { verdicts, dropped }
}
