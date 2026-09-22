//! Validation: what a proposal has to satisfy before a pass records it.
//!
//! The rule a family's answer must pass is local and checkable — no model, no network — so a model
//! can be wrong without being able to move a row.

use census_domain::model::{ReviewPacket, ReviewVerdict, ReviewVerdictKind, VerdictBatch};

use super::ReviewFamily;

/// A proposal validation admitted: the field and the value that may be recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted {
    pub field: String,
    pub value: String,
}

/// Why a proposal was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The proposal does not answer the field the family asked about.
    WrongField,
    /// The field's own rule refused the value.
    InvalidValue,
    /// The subject already carries a value for the field, so nothing needed proposing.
    AlreadyResolved,
}

/// Admit a proposal for a family's field, or say why not.
///
/// The rule is local and checkable: a jurisdiction proposal must name a state this census covers —
/// as a code or spelled out — and the packet must not already carry one, because a subject with a
/// jurisdiction is not the subject this family retained.
pub fn validate(
    family: ReviewFamily,
    verdict: &ReviewVerdict,
    packet: &ReviewPacket,
) -> Result<Admitted, Refusal> {
    let field = verdict
        .field
        .as_deref()
        .map(str::trim)
        .filter(|field| !field.is_empty())
        .ok_or(Refusal::WrongField)?;
    if field != family.field() {
        return Err(Refusal::WrongField);
    }
    let value = verdict
        .value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(Refusal::InvalidValue)?;
    match family {
        ReviewFamily::SchoolJurisdiction | ReviewFamily::MeetJurisdiction => {
            // Both families retain a subject with no jurisdiction at all, so a packet that already
            // carries one is not this case; a code and a spelled-out name are both answers.
            let Some(jurisdiction) = census_domain::UsJurisdiction::parse(value) else {
                return Err(Refusal::InvalidValue);
            };
            let already = packet
                .evidence
                .iter()
                .any(|fact| fact.field == "state" && !fact.value.trim().is_empty());
            if already {
                return Err(Refusal::AlreadyResolved);
            }
            Ok(Admitted {
                field: "state".to_string(),
                value: jurisdiction.code().to_string(),
            })
        }
    }
}

/// Read one batch into verdicts worth keeping: safe verdicts, each with the admitted proposal when
/// there is one, plus how many the reader dropped.
pub fn triage(
    packet: &ReviewPacket,
    family: ReviewFamily,
    batch: VerdictBatch,
) -> (Vec<(ReviewVerdict, Option<Admitted>)>, usize) {
    let (safe, dropped) = batch.sanitize(packet);
    let triaged = safe
        .into_iter()
        .map(|verdict| {
            let admitted = match verdict.kind {
                ReviewVerdictKind::ValueProposed => validate(family, &verdict, packet).ok(),
                ReviewVerdictKind::InsufficientEvidence => None,
            };
            (verdict, admitted)
        })
        .collect();
    (triaged, dropped)
}
