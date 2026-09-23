//! The athlete-identity family's own answer shape: what the lane may decide about two retained
//! athlete rows, and what reading one answer off the wire makes of it.
//!
//! The jurisdiction families ask for a value (a state code), so their answers are a field and a value
//! checked against the packet's own `state`. This family asks a question with exactly three answers
//! and the answer *is* the decision — whether two rows the merge kept apart are one person — so its
//! reader lives here and the jurisdiction rule never sees these answers. A family that asked for
//! "same person" in the slot a jurisdiction code occupies would validate as a value nothing can
//! check.
//!
//! One model, one case, one verdict: the lane asks a case once and records the answer it gave.

use census_domain::model::{ReviewPacket, ReviewVerdict, ReviewVerdictKind};

use super::families::IDENTITY_FIELD;
use super::verdicts::Refusal;

/// What the lane may answer about the two athlete rows a case compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AthleteVerdict {
    /// One person: the two canonical rows are one athlete observed through two rows.
    SamePerson,
    /// Two people: the rows collide on the merge key but are distinct athletes.
    DifferentPerson,
    /// The evidence does not decide; the case stays where it was rather than being closed.
    InsufficientEvidence,
}

impl AthleteVerdict {
    /// The slug the answer is written as: on the wire, and in the durable record.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::SamePerson => "same_person",
            Self::DifferentPerson => "different_person",
            Self::InsufficientEvidence => "insufficient_evidence",
        }
    }

    /// Parse an answer in the spellings models drift between.
    pub fn parse(value: &str) -> Option<Self> {
        let normalized = value
            .trim()
            .to_ascii_lowercase()
            .replace(['-', '_'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        match normalized.as_str() {
            "same person" | "sameperson" => Some(Self::SamePerson),
            "different person" | "differentperson" => Some(Self::DifferentPerson),
            "insufficient evidence" | "insufficientevidence" => Some(Self::InsufficientEvidence),
            _ => None,
        }
    }

    /// Whether this answer decides the case: only the two identity decisions close it.
    pub const fn decides(self) -> bool {
        matches!(self, Self::SamePerson | Self::DifferentPerson)
    }
}

/// Read one answer off the wire as this family's verdict, or say why it is not one.
///
/// Three local rules, and each refusal is returned rather than swallowed, so the caller can keep the
/// answer as the model gave it:
///
/// * the verdict names a case the packet asks about — a verdict for another case decides nothing;
/// * a proposal answers the field this family asks about, `identity`;
/// * the value is one of the three answers. A model that puts `insufficient_evidence` in the value
///   slot instead of the kind slot has still declined, so it reads as the decline it meant rather
///   than as an unparseable proposal.
pub(super) fn read(
    verdict: &ReviewVerdict,
    packet: &ReviewPacket,
) -> Result<AthleteVerdict, Refusal> {
    if !packet
        .cases
        .iter()
        .any(|case| case.case_id == verdict.case_id)
    {
        return Err(Refusal::UnnamedCase);
    }
    match verdict.kind {
        // The protocol's own way of declining: nothing proposed, nothing to check.
        ReviewVerdictKind::InsufficientEvidence => Ok(AthleteVerdict::InsufficientEvidence),
        ReviewVerdictKind::ValueProposed => {
            if verdict.field.as_deref().map(str::trim) != Some(IDENTITY_FIELD) {
                return Err(Refusal::WrongField);
            }
            let value = verdict.value.as_deref().map(str::trim).unwrap_or_default();
            AthleteVerdict::parse(value).ok_or(Refusal::InvalidValue)
        }
    }
}

#[cfg(test)]
#[path = "athlete_verdict_tests.rs"]
mod tests;
