use census_domain::model::{ReviewCase, ReviewPacket, ReviewVerdict, ReviewVerdictRecord};
use census_store::{StoreError, StoreResult};

use crate::ask::{ask_case, Answer};
use crate::packets::SubjectIndex;
use crate::{Adjudication, Admitted, ModelClient, ReviewFamily};

mod audit;
pub(super) use audit::{Audit, LaneAudit};

pub(super) const POLICY: &str = "dual-independent-review-v1";

pub(super) struct Asked {
    pub(super) packet: Option<ReviewPacket>,
    pub(super) answers: Option<[Answer; 2]>,
    pub(super) lanes: [LaneAudit; 2],
}

pub(super) struct Consensus {
    pub(super) admitted: Option<Admitted>,
    pub(super) reason: &'static str,
    pub(super) rejected: bool,
}

pub(super) fn invariant(detail: impl Into<String>) -> StoreError {
    StoreError::Invariant {
        detail: detail.into(),
    }
}

pub(super) fn independent(clients: &[ModelClient]) -> StoreResult<[&ModelClient; 2]> {
    let [first, second] = clients else {
        return Err(invariant(
            "review requires exactly two independent local endpoint lanes",
        ));
    };
    let port = |client: &ModelClient| {
        url::Url::parse(&client.options().completions_url())
            .map_err(|error| invariant(format!("invalid review lane URL: {error}")))?
            .port()
            .ok_or_else(|| invariant("review lane has no explicit port"))
    };
    if port(first)? == port(second)? {
        return Err(invariant(
            "review lanes must use distinct local server ports",
        ));
    }
    Ok([first, second])
}

pub(super) fn lane_binding(
    client: &ModelClient,
    packet: Option<&ReviewPacket>,
) -> StoreResult<LaneAudit> {
    let request_digest = match packet {
        Some(packet) => match crate::model::build_request_body(packet, client.options()) {
            Ok(body) => Some(
                census_domain::model::serialized_digest(&body)
                    .map_err(|error| invariant(format!("cannot bind review request: {error}")))?,
            ),
            Err(_) => None,
        },
        None => None,
    };
    Ok(LaneAudit::binding(client, request_digest))
}

pub(super) fn replay_matches(
    row: &ReviewVerdictRecord,
    packet: Option<&ReviewPacket>,
    lanes: &[LaneAudit; 2],
) -> bool {
    let Some(packet) = packet else {
        return false;
    };
    let Some(audit) = Audit::replay(row, packet) else {
        return false;
    };
    let Some(family) = ReviewFamily::from_label(&row.family) else {
        return false;
    };
    let [Some(first), Some(second)] = replay_lanes(&audit, packet, family, lanes) else {
        return false;
    };
    let asked = Asked {
        packet: audit.packet,
        answers: Some([first, second]),
        lanes: audit.lanes,
    };
    let consensus = decide(&asked, Some(packet));
    let accepted = consensus.admitted.is_some();
    let matches_value = consensus.admitted.as_ref().map_or_else(
        || row.field.is_empty() && row.value.is_empty(),
        |admitted| row.field == admitted.field && row.value == admitted.value,
    );
    row.accepted == accepted
        && matches_value
        && audit.outcome == consensus.reason
        && row.kind
            == if accepted {
                "value_proposed"
            } else {
                "insufficient_evidence"
            }
        && row.confidence
            == if accepted {
                crate::records::confidence(&asked)
            } else {
                0
            }
        && row.reviewer == "dual-independent-consensus"
}

fn replay_lanes(
    audit: &Audit,
    packet: &ReviewPacket,
    family: ReviewFamily,
    lanes: &[LaneAudit; 2],
) -> [Option<Answer>; 2] {
    lanes.each_ref().map(|binding| {
        audit
            .lanes
            .iter()
            .find(|lane| lane.same_binding(binding))
            .and_then(|lane| lane.replay_answer(packet, family))
    })
}

pub(super) async fn ask_lanes(
    case: &ReviewCase,
    family: ReviewFamily,
    subjects: &SubjectIndex,
    clients: [&ModelClient; 2],
    previous: Option<&ReviewVerdictRecord>,
) -> StoreResult<Asked> {
    let packet = subjects.packet(case, family)?;
    let [first_client, second_client] = clients;
    let lanes = [
        lane_binding(first_client, packet.as_ref())?,
        lane_binding(second_client, packet.as_ref())?,
    ];
    let cached = packet
        .as_ref()
        .and_then(|packet| {
            previous
                .and_then(|row| Audit::replay(row, packet))
                .map(|audit| replay_lanes(&audit, packet, family, &lanes))
        })
        .map_or([None, None], |answers| answers);
    let answers = match packet.as_ref() {
        Some(packet) => {
            let [first_cached, second_cached] = cached;
            let (first, second) = futures::future::join(
                ask_or_replay(first_client, packet, family, first_cached),
                ask_or_replay(second_client, packet, family, second_cached),
            )
            .await;
            Some([first, second])
        }
        None => None,
    };
    Ok(Asked {
        packet,
        answers,
        lanes,
    })
}

async fn ask_or_replay(
    client: &ModelClient,
    packet: &ReviewPacket,
    family: ReviewFamily,
    cached: Option<Answer>,
) -> Answer {
    match cached {
        Some(answer) => answer,
        None => ask_case(client, packet, family).await,
    }
}

fn usable(answer: &Answer) -> Option<(&ReviewVerdict, &Adjudication)> {
    match answer {
        Answer::Answered {
            batch,
            verdicts,
            dropped: 0,
        } if batch.verdicts.len() == 1
            && batch.verdicts.iter().all(|row| row.confidence <= 100) =>
        {
            let [(verdict, adjudication)] = verdicts.as_slice() else {
                return None;
            };
            Some((verdict, adjudication))
        }
        Answer::Answered { .. } | Answer::Failed(_) => None,
    }
}

fn unresolved(reason: &'static str, rejected: bool) -> Consensus {
    Consensus {
        admitted: None,
        reason,
        rejected,
    }
}

pub(super) fn decide(asked: &Asked, current: Option<&ReviewPacket>) -> Consensus {
    if asked.packet.as_ref() != current {
        return unresolved("evidence_changed", false);
    }
    let Some(_) = asked.packet.as_ref() else {
        return unresolved("missing_subject", false);
    };
    let Some([first, second]) = asked.answers.as_ref() else {
        return unresolved("missing_advice", false);
    };
    if matches!(first, Answer::Failed(_)) || matches!(second, Answer::Failed(_)) {
        return unresolved("lane_failed", false);
    }
    let (Some((_, first)), Some((_, second))) = (usable(first), usable(second)) else {
        return unresolved("malformed_or_missing_advice", false);
    };
    match (first, second) {
        (Adjudication::Decided(first), Adjudication::Decided(second)) if first == second => {
            Consensus {
                admitted: Some(first.clone()),
                reason: "agreement",
                rejected: false,
            }
        }
        (Adjudication::Refused(crate::Refusal::HardContradiction(_)), _)
        | (_, Adjudication::Refused(crate::Refusal::HardContradiction(_))) => {
            unresolved("hard_contradiction", true)
        }
        (Adjudication::Refused(_), _) | (_, Adjudication::Refused(_)) => {
            unresolved("refused", true)
        }
        (Adjudication::Undecided, _) | (_, Adjudication::Undecided) => {
            unresolved("insufficient_evidence", false)
        }
        (Adjudication::Decided(_), Adjudication::Decided(_)) => unresolved("disagreement", false),
    }
}

#[cfg(test)]
pub(crate) mod tests;
