use census_domain::model::{ReviewPacket, ReviewVerdict, ReviewVerdictKind};

use super::families::IDENTITY_FIELD;
use super::verdicts::Refusal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AthleteVerdict {
    SamePerson,
    DifferentPerson,
    InsufficientEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardContradiction {
    GradYearEvidenceDiffers,
    GenderDiffers,
}

impl HardContradiction {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::GradYearEvidenceDiffers => "grad_year_evidence_differs",
            Self::GenderDiffers => "gender_differs",
        }
    }
}

impl Refusal {
    pub const fn rationale(self) -> Option<&'static str> {
        match self {
            Self::HardContradiction(flag) => Some(flag.slug()),
            Self::WrongField | Self::InvalidValue | Self::AlreadyResolved | Self::UnnamedCase => {
                None
            }
        }
    }
}

impl AthleteVerdict {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::SamePerson => "same_person",
            Self::DifferentPerson => "different_person",
            Self::InsufficientEvidence => "insufficient_evidence",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "same_person" => Some(Self::SamePerson),
            "different_person" => Some(Self::DifferentPerson),
            "insufficient_evidence" => Some(Self::InsufficientEvidence),
            _ => None,
        }
    }

    pub const fn decides(self) -> bool {
        matches!(self, Self::SamePerson | Self::DifferentPerson)
    }
}
fn has_positive_evidence(packet: &ReviewPacket) -> bool {
    packet
        .evidence
        .iter()
        .filter(|fact| fact.field == "flag")
        .any(|fact| fact.value.starts_with("shared_source_identity:"))
}

fn hard_contradiction(packet: &ReviewPacket) -> Option<HardContradiction> {
    packet
        .evidence
        .iter()
        .filter(|fact| fact.field == "flag")
        .find_map(|fact| {
            let flag = fact.value.split_once(':')?.0.trim();
            match flag {
                "grad_year_evidence_differs" => Some(HardContradiction::GradYearEvidenceDiffers),
                "gender_differs" => Some(HardContradiction::GenderDiffers),
                _ => None,
            }
        })
}

fn read_answer(value: &str, packet: &ReviewPacket) -> Result<AthleteVerdict, Refusal> {
    let Some(answer) = AthleteVerdict::parse(value) else {
        return Err(Refusal::InvalidValue);
    };
    if answer != AthleteVerdict::SamePerson {
        return Ok(answer);
    }
    if !has_positive_evidence(packet) {
        return Err(Refusal::InvalidValue);
    }
    match hard_contradiction(packet) {
        Some(flag) => Err(Refusal::HardContradiction(flag)),
        None => Ok(answer),
    }
}

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
        ReviewVerdictKind::InsufficientEvidence => Ok(AthleteVerdict::InsufficientEvidence),
        ReviewVerdictKind::ValueProposed => {
            if verdict.field.as_deref().map(str::trim) != Some(IDENTITY_FIELD) {
                return Err(Refusal::WrongField);
            }
            let value = verdict.value.as_deref().map(str::trim).unwrap_or_default();
            read_answer(value, packet)
        }
    }
}

#[cfg(test)]
#[path = "athlete_verdict_tests.rs"]
mod tests;
