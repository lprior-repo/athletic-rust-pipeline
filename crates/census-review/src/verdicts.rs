use census_domain::model::{ReviewPacket, ReviewVerdict, ReviewVerdictKind, VerdictBatch};

use super::families::IDENTITY_FIELD;
use super::{athlete_verdict, ReviewFamily};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted {
    pub field: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    WrongField,
    InvalidValue,
    AlreadyResolved,
    UnnamedCase,
    HardContradiction(crate::athlete_verdict::HardContradiction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Adjudication {
    Decided(Admitted),
    Undecided,
    Refused(Refusal),
}

impl Adjudication {
    pub const fn admitted(&self) -> Option<&Admitted> {
        match self {
            Self::Decided(admitted) => Some(admitted),
            Self::Undecided | Self::Refused(_) => None,
        }
    }
}

pub fn validate(
    family: ReviewFamily,
    verdict: &ReviewVerdict,
    packet: &ReviewPacket,
) -> Adjudication {
    match family {
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

fn named(slot: &Option<String>) -> Option<&str> {
    slot.as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

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
