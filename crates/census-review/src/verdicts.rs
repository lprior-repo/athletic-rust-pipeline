use census_domain::model::{Gender, ReviewPacket, ReviewVerdict, ReviewVerdictKind, VerdictBatch};
use census_domain::review_decision::{Adjudication, Admitted, Refusal};

use super::families::{GENDER_FIELD, IDENTITY_FIELD, SCHOOL_ADDRESS_FIELD};
use super::{athlete_verdict, ReviewFamily};

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
        ReviewFamily::SchoolLink => link(verdict),
        ReviewFamily::AthleteGender => gender(verdict),
        ReviewFamily::SchoolAddress => address(verdict),
        ReviewFamily::TransferAmbiguous => Adjudication::Undecided,
    }
}

fn gender(verdict: &ReviewVerdict) -> Adjudication {
    if named(&verdict.field) != Some(GENDER_FIELD) {
        return Adjudication::Refused(Refusal::WrongField);
    }
    let Some(value) = named(&verdict.value) else {
        return Adjudication::Refused(Refusal::InvalidValue);
    };
    let parsed = Gender::parse_milesplit(value);
    match parsed {
        Gender::Boys | Gender::Girls => Adjudication::Decided(Admitted {
            field: GENDER_FIELD.to_string(),
            value: parsed.stable_key().to_string(),
        }),
        _ => Adjudication::Refused(Refusal::InvalidValue),
    }
}

fn address(verdict: &ReviewVerdict) -> Adjudication {
    if named(&verdict.field) != Some(SCHOOL_ADDRESS_FIELD) {
        return Adjudication::Refused(Refusal::WrongField);
    }
    let Some(value) = named(&verdict.value) else {
        return Adjudication::Refused(Refusal::InvalidValue);
    };
    if value.len() > 200 {
        return Adjudication::Refused(Refusal::InvalidValue);
    }
    Adjudication::Decided(Admitted {
        field: SCHOOL_ADDRESS_FIELD.to_string(),
        value: value.to_string(),
    })
}

fn link(verdict: &ReviewVerdict) -> Adjudication {
    if named(&verdict.field) != Some(IDENTITY_FIELD) {
        return Adjudication::Refused(Refusal::WrongField);
    }
    let Some(value) = named(&verdict.value) else {
        return Adjudication::Refused(Refusal::InvalidValue);
    };
    match identity_label(value) {
        Some(label) => Adjudication::Decided(Admitted {
            field: IDENTITY_FIELD.to_string(),
            value: label,
        }),
        None => Adjudication::Refused(Refusal::InvalidValue),
    }
}

fn identity_label(value: &str) -> Option<String> {
    use census_domain::school_directory::{NcesSchoolId, PssId, StateRecordId};
    let (kind, rest) = value.split_once(':')?;
    match kind {
        "nces" => NcesSchoolId::parse(rest)
            .ok()
            .map(|id| format!("nces:{}", id.as_str())),
        "pss" => PssId::parse(rest)
            .ok()
            .map(|id| format!("pss:{}", id.as_str())),
        "state" => {
            let (code, id) = rest.split_once(':')?;
            let state = census_domain::UsJurisdiction::parse(code)?;
            StateRecordId::parse(id)
                .ok()
                .map(|id| format!("state:{}:{}", state.code(), id.as_str()))
        }
        _ => None,
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
