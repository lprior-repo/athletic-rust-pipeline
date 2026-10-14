use super::bounds::check_limit;
use super::equality::same_capture;
use super::{
    AffiliationCandidateError, CandidateQuery, ChangeKind, KnownBounds, LocallyCheckedAssertion,
    LocallyCheckedChange, LocallyCheckedEvidence, PersonCandidate, RelationshipCandidate, Resource,
    TemporalScope,
};

pub(super) fn check_person(
    person: PersonCandidate<'_>,
    query: &CandidateQuery<'_>,
) -> Result<(), AffiliationCandidateError> {
    if let PersonCandidate::Mapped { known, .. } = person {
        require_visible(known, query)?;
    }
    Ok(())
}

pub(super) fn require_visible(
    known: KnownBounds<'_>,
    query: &CandidateQuery<'_>,
) -> Result<(), AffiliationCandidateError> {
    if !known.visible(query.known.lineage, query.known.revision)? {
        return Err(AffiliationCandidateError::DependencyNotVisible);
    }
    Ok(())
}

pub(super) fn check_assertion(
    fact: &LocallyCheckedAssertion<'_>,
    query: &CandidateQuery<'_>,
) -> Result<(), AffiliationCandidateError> {
    let raw = fact.raw();
    let visible = raw
        .known
        .visible(query.known.lineage, query.known.revision)?;
    raw.owner
        .known
        .visible(query.known.lineage, query.known.revision)?;
    if let PersonCandidate::Mapped { known, .. } = raw.person {
        known.visible(query.known.lineage, query.known.revision)?;
    }
    if visible {
        check_person(raw.person, query)?;
        require_visible(raw.owner.known, query)?;
    }
    if let RelationshipCandidate::RepresentsCoopProgram {
        member: super::CoopMemberCandidate::Claimed { known, .. },
        ..
    } = raw.relationship
    {
        known.visible(query.known.lineage, query.known.revision)?;
        if visible {
            require_visible(known, query)?;
        }
    }
    if let TemporalScope::AcademicSeason { calendar, .. } = raw.temporal {
        calendar
            .known
            .visible(query.known.lineage, query.known.revision)?;
        if visible {
            require_visible(calendar.known, query)?;
        }
    }
    Ok(())
}

pub(super) fn check_change(
    change: &LocallyCheckedChange<'_>,
    assertions: &[&LocallyCheckedAssertion<'_>],
    query: &CandidateQuery<'_>,
) -> Result<(), AffiliationCandidateError> {
    let raw = change.raw();
    let visible = raw
        .known
        .visible(query.known.lineage, query.known.revision)?;
    let target = assertions
        .iter()
        .find(|fact| fact.raw().id == raw.target)
        .ok_or(AffiliationCandidateError::DanglingOrForeignChangeTarget)?;
    match (raw.person, target.raw().person) {
        (
            PersonCandidate::Mapped {
                person: left,
                athlete: a,
                ..
            },
            PersonCandidate::Mapped {
                person: right,
                athlete: b,
                ..
            },
        ) if left == right && a == b => {}
        _ => return Err(AffiliationCandidateError::DanglingOrForeignChangeTarget),
    }
    if let PersonCandidate::Mapped { known, .. } = raw.person {
        known.visible(query.known.lineage, query.known.revision)?;
    }
    if visible {
        check_person(raw.person, query)?;
        match raw.kind {
            ChangeKind::WithdrawsClaim { .. } | ChangeKind::EndsRelationship { .. } => {}
            ChangeKind::Corrects { .. }
            | ChangeKind::SourceActivation { .. }
            | ChangeKind::ReviewedResolution { .. } => {
                return Err(AffiliationCandidateError::UnsupportedChange)
            }
        }
    }
    Ok(())
}

pub(super) fn check_evidence<'a>(
    evidence: &'a LocallyCheckedEvidence<'a>,
    seen: &mut Vec<&'a LocallyCheckedEvidence<'a>>,
    bytes: &mut usize,
    query: &CandidateQuery<'_>,
) -> Result<(), AffiliationCandidateError> {
    let raw = evidence.raw();
    if raw.acquired_at > query.known.captured_at {
        return Err(AffiliationCandidateError::AcquisitionAfterFrozenInstant);
    }
    if raw.acquired_at > query.acquisition_cutoff {
        return Err(AffiliationCandidateError::AcquisitionAfterDeclaredCutoff);
    }
    let mut capture_seen = false;
    let mut interpretation_seen = false;
    for prior in seen.iter() {
        let previous = prior.raw();
        if previous.occurrence == raw.occurrence || previous.manifest_key == raw.manifest_key {
            if !same_capture(previous, raw) {
                return Err(AffiliationCandidateError::DuplicateIdDifferentPayload);
            }
            capture_seen = true;
        }
        if previous.interpretation_key == raw.interpretation_key {
            if *prior != evidence {
                return Err(AffiliationCandidateError::DuplicateIdDifferentPayload);
            }
            interpretation_seen = true;
        }
    }
    if capture_seen && interpretation_seen {
        return Ok(());
    }
    let capture = if capture_seen {
        0
    } else {
        evidence.capture_byte_charge()?
    };
    let interpretation = if interpretation_seen {
        0
    } else {
        raw.interpretation_bytes.len()
    };
    *bytes = bytes
        .checked_add(capture)
        .and_then(|total| total.checked_add(interpretation))
        .ok_or(AffiliationCandidateError::CounterOverflow)?;
    check_limit(Resource::ComponentBytes, *bytes, query.grant.bytes)?;
    seen.push(evidence);
    Ok(())
}
