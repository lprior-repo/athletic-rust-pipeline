//! Validation: what a proposal has to satisfy before a pass records it.
//!
//! The rule a family's answer must pass is local and checkable — no model, no network — so a model
//! can be wrong without being able to move a row. Every family answers its own kind of question, so
//! the rule is per family: the jurisdiction families ask for a value, the athlete family asks for a
//! decision, and [`Adjudication`] is the shape both produce. A refusal is never a silence: it is
//! returned with the reason, and the caller keeps the answer the model gave.

use census_domain::model::{ReviewPacket, ReviewVerdict, ReviewVerdictKind, VerdictBatch};

use super::families::IDENTITY_FIELD;
use super::{athlete_verdict, ReviewFamily};

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
    /// The verdict does not name a case the packet asks about, so it decides nothing.
    UnnamedCase,
    /// A `same_person` proposal contradicts deterministic packet evidence.
    HardContradiction(crate::athlete_verdict::HardContradiction),
}

/// What validating one proposal produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Adjudication {
    /// A decision, and the value that may be recorded for it.
    Decided(Admitted),
    /// The answer was not a decision: the lane declined, so there is no value to record. An athlete
    /// case is retained as terminal for this evidence snapshot, and new evidence mints a new case.
    Undecided,
    /// The proposal was refused, and why. The case stays for an operator, and the answer is kept as
    /// the model gave it rather than dropped.
    Refused(Refusal),
}

impl Adjudication {
    /// The value validation admitted, when it admitted one.
    pub const fn admitted(&self) -> Option<&Admitted> {
        match self {
            Self::Decided(admitted) => Some(admitted),
            Self::Undecided | Self::Refused(_) => None,
        }
    }
}

/// Adjudicate one proposal against its family's own rule.
pub fn validate(
    family: ReviewFamily,
    verdict: &ReviewVerdict,
    packet: &ReviewPacket,
) -> Adjudication {
    match family {
        // The athlete family answers with a decision rather than a value, so it has its own reader
        // and never reaches the jurisdiction rule below.
        ReviewFamily::AthleteIdentity => match athlete_verdict::read(verdict, packet) {
            Ok(answer) if answer.decides() => Adjudication::Decided(Admitted {
                field: IDENTITY_FIELD.to_string(),
                value: answer.slug().to_string(),
            }),
            Ok(_) => Adjudication::Undecided,
            Err(reason) => Adjudication::Refused(reason),
        },
        ReviewFamily::SchoolJurisdiction | ReviewFamily::MeetJurisdiction => {
            jurisdiction(verdict, packet, family.field())
        }
    }
}

/// The jurisdiction rule: a proposal must answer `state` with a jurisdiction this census covers.
///
/// Both jurisdiction families retain a subject with no jurisdiction at all, so a packet that already
/// carries one is not this case; a code and a spelled-out name are both answers.
fn jurisdiction(verdict: &ReviewVerdict, packet: &ReviewPacket, field: &str) -> Adjudication {
    if named(&verdict.field) != Some(field) {
        return Adjudication::Refused(Refusal::WrongField);
    }
    let Some(value) = named(&verdict.value) else {
        return Adjudication::Refused(Refusal::InvalidValue);
    };
    let Some(jurisdiction) = census_domain::UsJurisdiction::parse(value) else {
        return Adjudication::Refused(Refusal::InvalidValue);
    };
    let already = packet
        .evidence
        .iter()
        .any(|fact| fact.field == field && !fact.value.trim().is_empty());
    if already {
        return Adjudication::Refused(Refusal::AlreadyResolved);
    }
    Adjudication::Decided(Admitted {
        field: field.to_string(),
        value: jurisdiction.code().to_string(),
    })
}

/// A slot's text, when the model filled it in.
fn named(slot: &Option<String>) -> Option<&str> {
    slot.as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

/// Read one batch into adjudications worth keeping, plus how many the reader dropped.
///
/// Only a proposal is validated: a verdict that declined carries nothing to check, and the caller
/// reads that as the case not being closed.
pub fn triage(
    packet: &ReviewPacket,
    family: ReviewFamily,
    batch: VerdictBatch,
) -> (Vec<(ReviewVerdict, Adjudication)>, usize) {
    let (safe, dropped) = batch.sanitize(packet);
    let triaged = safe
        .into_iter()
        .map(|verdict| {
            let adjudication = match verdict.kind {
                ReviewVerdictKind::ValueProposed => validate(family, &verdict, packet),
                ReviewVerdictKind::InsufficientEvidence => Adjudication::Undecided,
            };
            (verdict, adjudication)
        })
        .collect();
    (triaged, dropped)
}
