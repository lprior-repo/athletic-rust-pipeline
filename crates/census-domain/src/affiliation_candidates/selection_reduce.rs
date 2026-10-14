use super::relationship::{same_relationship, school};
use super::{
    AffiliationCandidateError, CandidateOutcome, CandidateQuery, ChangeKind, CoopMemberCandidate,
    Disposition, FactDisposition, LocallyCheckedAssertion, LocallyCheckedChange, PersonCandidate,
    RelationshipCandidate, Resource, SubjectGap,
};
use crate::model::ProfileDate;

pub(super) fn classify_assertion(
    fact: &LocallyCheckedAssertion<'_>,
    changes: &[&LocallyCheckedChange<'_>],
    query: &CandidateQuery<'_>,
) -> Result<Disposition, AffiliationCandidateError> {
    let raw = fact.raw();
    if !raw
        .known
        .visible(query.known.lineage, query.known.revision)?
    {
        return Ok(Disposition::NotVisibleAtKnownRevision);
    }
    if !raw.sport.applies(query.sport) {
        return Ok(Disposition::OutsideSport);
    }
    match raw.person {
        PersonCandidate::MissingKey => {
            return Ok(Disposition::SubjectUnresolved(SubjectGap::MissingPersonKey))
        }
        PersonCandidate::Unbound { gap, .. } => return Ok(Disposition::SubjectUnresolved(gap)),
        PersonCandidate::Mapped { athlete, .. } if athlete != query.athlete => {
            return Ok(Disposition::DifferentAthlete)
        }
        PersonCandidate::Mapped { .. } => {}
    }
    let changed = changed_disposition(raw.id, changes, query)?;
    if let Some(disposition) = changed {
        return Ok(disposition);
    }
    match raw.temporal.applies(query.as_of) {
        Ok(false) if raw.temporal.ended_before(query.as_of) => {
            return Ok(Disposition::ExplicitIntervalEnded)
        }
        Ok(false) => return Ok(Disposition::OutsideEffectiveScope),
        Err(gap) => return Ok(Disposition::TemporalUnresolved(gap)),
        Ok(true) => {}
    }
    if matches!(
        raw.relationship,
        RelationshipCandidate::RepresentsCoopProgram {
            member: CoopMemberCandidate::Unknown,
            ..
        }
    ) {
        return Ok(Disposition::CoopMemberUnknown);
    }
    if let RelationshipCandidate::RepresentsCoopProgram {
        member:
            CoopMemberCandidate::Claimed {
                campus_binding_interval,
                program_membership_interval,
                ..
            },
        ..
    } = raw.relationship
    {
        if !campus_binding_interval.contains(query.as_of)
            || !program_membership_interval.contains(query.as_of)
        {
            return Ok(Disposition::CoopMemberUnknown);
        }
    }
    Ok(Disposition::Candidate)
}

fn changed_disposition(
    id: super::ObjectRef<'_>,
    changes: &[&LocallyCheckedChange<'_>],
    query: &CandidateQuery<'_>,
) -> Result<Option<Disposition>, AffiliationCandidateError> {
    let mut result = None;
    for change in changes {
        let raw = change.raw();
        if raw.target != id
            || !raw
                .known
                .visible(query.known.lineage, query.known.revision)?
        {
            continue;
        }
        if !change_applies(raw.kind, query.as_of)? {
            continue;
        }
        let disposition = match raw.kind {
            ChangeKind::WithdrawsClaim { .. } => Disposition::WithdrawnClaim,
            ChangeKind::EndsRelationship { .. } => Disposition::RelationshipEnded,
            _ => return Err(AffiliationCandidateError::UnsupportedChange),
        };
        if result.is_some_and(|previous| previous != disposition) {
            return Err(AffiliationCandidateError::ConflictingChanges);
        }
        result = Some(disposition);
    }
    Ok(result)
}

pub(super) fn classify_change(
    change: &LocallyCheckedChange<'_>,
    assertions: &[LocallyCheckedAssertion<'_>],
    query: &CandidateQuery<'_>,
) -> Result<Disposition, AffiliationCandidateError> {
    let raw = change.raw();
    if !raw
        .known
        .visible(query.known.lineage, query.known.revision)?
    {
        return Ok(Disposition::NotVisibleAtKnownRevision);
    }
    let target = assertions
        .iter()
        .find(|fact| fact.raw().id == raw.target)
        .ok_or(AffiliationCandidateError::DanglingOrForeignChangeTarget)?
        .raw();
    if !target
        .known
        .visible(query.known.lineage, query.known.revision)?
    {
        return Ok(Disposition::ChangeTargetNotVisible);
    }
    if let PersonCandidate::Mapped { athlete, .. } = raw.person {
        if athlete != query.athlete {
            return Ok(Disposition::DifferentAthlete);
        }
    }
    if !target.sport.applies(query.sport) {
        return Ok(Disposition::OutsideSport);
    }
    if change_applies(raw.kind, query.as_of)? {
        return Ok(Disposition::ChangeApplied);
    }
    Ok(Disposition::ChangeOutsideEffectiveScope)
}

fn change_applies(
    kind: ChangeKind<'_>,
    as_of: &ProfileDate,
) -> Result<bool, AffiliationCandidateError> {
    match kind {
        ChangeKind::WithdrawsClaim { scope } => Ok(scope.contains(as_of)),
        ChangeKind::EndsRelationship { last_day } => Ok(as_of > last_day),
        ChangeKind::Corrects { .. }
        | ChangeKind::SourceActivation { .. }
        | ChangeKind::ReviewedResolution { .. } => {
            Err(AffiliationCandidateError::UnsupportedChange)
        }
    }
}

#[derive(Clone, Copy)]
enum CandidateState<'a> {
    Empty,
    One(&'a LocallyCheckedAssertion<'a>),
    Compatible(&'a LocallyCheckedAssertion<'a>),
    Conflicted,
}

#[derive(Clone, Copy)]
enum Obstruction {
    Clear,
    Unknown,
    Subject,
}

pub(super) fn reduce<'a>(
    facts: &[FactDisposition<'a>],
) -> Result<CandidateOutcome<'a>, AffiliationCandidateError> {
    let mut state = CandidateState::Empty;
    let mut obstruction = Obstruction::Clear;
    let mut ended = false;
    let mut targets: usize = 0;
    for (index, fact) in facts.iter().enumerate() {
        let FactDisposition::Assertion { fact, disposition } = fact else {
            continue;
        };
        match disposition {
            Disposition::Candidate => {
                if !prior_equivalent(facts, index, fact.raw().relationship) {
                    targets = targets
                        .checked_add(1)
                        .ok_or(AffiliationCandidateError::CounterOverflow)?;
                    super::bounds::check_limit(Resource::Targets, targets, 64)?;
                }
                state = add_candidate(state, fact);
            }
            Disposition::SubjectUnresolved(_) => obstruction = Obstruction::Subject,
            Disposition::TemporalUnresolved(_) | Disposition::CoopMemberUnknown => {
                if matches!(obstruction, Obstruction::Clear) {
                    obstruction = Obstruction::Unknown;
                }
            }
            Disposition::RelationshipEnded | Disposition::ExplicitIntervalEnded => ended = true,
            _ => {}
        }
    }
    Ok(finish(state, obstruction, ended))
}

fn prior_equivalent(
    facts: &[FactDisposition<'_>],
    through_exclusive: usize,
    relationship: RelationshipCandidate<'_>,
) -> bool {
    facts.iter().take(through_exclusive).any(|fact| match fact {
        FactDisposition::Assertion {
            fact,
            disposition: Disposition::Candidate,
        } => same_relationship(fact.raw().relationship, relationship),
        _ => false,
    })
}

fn add_candidate<'a>(
    state: CandidateState<'a>,
    fact: &'a LocallyCheckedAssertion<'a>,
) -> CandidateState<'a> {
    match state {
        CandidateState::Empty => CandidateState::One(fact),
        CandidateState::Conflicted => CandidateState::Conflicted,
        CandidateState::One(first) | CandidateState::Compatible(first) => {
            let left = first.raw().relationship;
            let right = fact.raw().relationship;
            if same_relationship(left, right) {
                return state;
            }
            match (school(left), school(right)) {
                (Some(a), Some(b)) if a == b => CandidateState::Compatible(first),
                _ => CandidateState::Conflicted,
            }
        }
    }
}

fn finish<'a>(
    state: CandidateState<'a>,
    obstruction: Obstruction,
    ended: bool,
) -> CandidateOutcome<'a> {
    if matches!(state, CandidateState::Conflicted) {
        return CandidateOutcome::Conflicted;
    }
    match obstruction {
        Obstruction::Subject => return CandidateOutcome::SubjectUnresolved,
        Obstruction::Unknown => return CandidateOutcome::Unknown,
        Obstruction::Clear => {}
    }
    match state {
        CandidateState::One(first) => CandidateOutcome::SingleCandidate(first),
        CandidateState::Compatible(_) => CandidateOutcome::MultipleCompatibleCandidates,
        CandidateState::Empty if ended => CandidateOutcome::EndedWithoutReplacement,
        CandidateState::Empty => CandidateOutcome::Unknown,
        CandidateState::Conflicted => CandidateOutcome::Conflicted,
    }
}
