//! Asking one retained case: the request, the batch that came back, and how it was triaged.

use census_domain::model::{ReviewCase, ReviewVerdict};

use super::model::{ModelClient, ModelError};
use super::packets::SubjectIndex;
use super::verdicts::{triage, Admitted};
use super::ReviewFamily;

/// What asking one retained case produced.
pub(super) enum Answer {
    /// The store no longer holds the case's subject.
    NoSubject,
    /// The model request failed outright.
    Failed(ModelError),
    /// The model answered: the verdicts worth keeping, and how many the batch dropped.
    Answered {
        verdicts: Vec<(ReviewVerdict, Option<Admitted>)>,
        dropped: usize,
    },
}

/// Ask one case's subject and triage the batch that came back.
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
