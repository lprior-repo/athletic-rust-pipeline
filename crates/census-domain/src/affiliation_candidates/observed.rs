use super::{
    AffiliationCandidateError, KnownBounds, LocallyCheckedEvidence, ObjectRef, SourceSpan,
    SportScope, TemporalScope,
};
use crate::model::{AthleteId, SchoolId, SourceSubject, TeamId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubjectGap {
    MissingPersonKey,
    UnacceptedBinding,
    ProviderIdReused,
    RevokedOrAmbiguousIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonCandidate<'a> {
    MissingKey,
    Unbound {
        person: &'a SourceSubject,
        gap: SubjectGap,
    },
    Mapped {
        person: &'a SourceSubject,
        athlete: &'a AthleteId,
        decision: ObjectRef<'a>,
        known: KnownBounds<'a>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnerTarget<'a> {
    School(&'a SchoolId),
    Program(&'a TeamId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedOwner<'a> {
    Unknown,
    Known(&'a SourceSubject),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawOwnership<'a> {
    pub requested: &'a SourceSubject,
    pub observed: ObservedOwner<'a>,
    pub published: &'a [&'a SourceSubject],
    pub governing: &'a SourceSubject,
    pub target: OwnerTarget<'a>,
    pub binding_decision: ObjectRef<'a>,
    pub known: KnownBounds<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopMemberCandidate<'a> {
    Unknown,
    Claimed {
        school: &'a SchoolId,
        athlete_member_binding: ObjectRef<'a>,
        source_person: &'a SourceSubject,
        campus_binding_interval: &'a crate::model::ProfileDateRangeV1,
        program_membership_interval: &'a crate::model::ProfileDateRangeV1,
        membership_statement: SourceSpan,
        known: KnownBounds<'a>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonSchoolKind {
    Independent,
    HomeSchool,
    ClubOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationshipCandidate<'a> {
    EnrolledAtSchool(&'a SchoolId),
    RepresentsSchoolProgram {
        school: &'a SchoolId,
        program: &'a TeamId,
    },
    RepresentsCoopProgram {
        program: &'a TeamId,
        member: CoopMemberCandidate<'a>,
    },
    NonSchool(NonSchoolKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawAssertion<'a> {
    pub id: ObjectRef<'a>,
    pub evidence: &'a LocallyCheckedEvidence<'a>,
    pub roster_qualification_claim: ObjectRef<'a>,
    pub source_activation_claim: ObjectRef<'a>,
    pub row_locator: ObjectRef<'a>,
    pub person: PersonCandidate<'a>,
    pub owner: RawOwnership<'a>,
    pub relationship: RelationshipCandidate<'a>,
    pub temporal: TemporalScope<'a>,
    pub sport: SportScope,
    pub known: KnownBounds<'a>,
    pub membership_statement: SourceSpan,
    pub person_statement: SourceSpan,
    pub owner_statement: SourceSpan,
    pub temporal_statement: SourceSpan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocallyCheckedAssertion<'a> {
    raw: RawAssertion<'a>,
}

impl<'a> LocallyCheckedAssertion<'a> {
    pub fn check(raw: RawAssertion<'a>) -> Result<Self, AffiliationCandidateError> {
        check_ownership(&raw)?;
        check_relationship(raw.relationship, raw.owner.target, raw.evidence)?;
        if let RelationshipCandidate::RepresentsCoopProgram {
            member: CoopMemberCandidate::Claimed { source_person, .. },
            ..
        } = raw.relationship
        {
            match raw.person {
                PersonCandidate::Mapped { person, .. }
                | PersonCandidate::Unbound { person, .. }
                    if person == source_person => {}
                _ => return Err(AffiliationCandidateError::CoopPersonMismatch),
            }
        }
        if let PersonCandidate::Mapped { athlete, .. } = raw.person {
            if athlete.as_str().is_empty() || athlete.as_str().len() > 512 {
                return Err(AffiliationCandidateError::InvalidExistingId);
            }
        }
        raw.sport.check()?;
        raw.temporal.check(raw.sport, raw.evidence)?;
        for span in [
            raw.membership_statement,
            raw.person_statement,
            raw.owner_statement,
            raw.temporal_statement,
        ] {
            span.read(raw.evidence)?;
        }
        Ok(Self { raw })
    }

    pub const fn raw(&self) -> &RawAssertion<'a> {
        &self.raw
    }
}

fn check_ownership(raw: &RawAssertion<'_>) -> Result<(), AffiliationCandidateError> {
    let owner = raw.owner;
    if owner.published.is_empty() {
        return Err(AffiliationCandidateError::MissingPublishedOwner);
    }
    super::bounds::check_limit(super::Resource::PublishedOwners, owner.published.len(), 32)?;
    if let Some(first) = owner.published.first() {
        if owner.published.iter().any(|published| published != first) {
            return Err(AffiliationCandidateError::ConflictingAuthoritativeOwners);
        }
    }
    if owner.requested != owner.governing
        || raw.evidence.raw().owner != owner.governing
        || owner
            .published
            .iter()
            .any(|published| *published != owner.governing)
    {
        return Err(AffiliationCandidateError::ForeignOwner);
    }
    if let ObservedOwner::Known(observed) = owner.observed {
        if observed != owner.governing {
            return Err(AffiliationCandidateError::ForeignOwner);
        }
    }
    Ok(())
}

fn check_relationship(
    relationship: RelationshipCandidate<'_>,
    owner: OwnerTarget<'_>,
    evidence: &LocallyCheckedEvidence<'_>,
) -> Result<(), AffiliationCandidateError> {
    match (relationship, owner) {
        (RelationshipCandidate::EnrolledAtSchool(school), OwnerTarget::School(bound))
            if school == bound =>
        {
            check_school(school)
        }
        (
            RelationshipCandidate::RepresentsSchoolProgram { school, program },
            OwnerTarget::School(bound),
        ) if school == bound => {
            check_school(school)?;
            check_program(program)
        }
        (
            RelationshipCandidate::RepresentsCoopProgram { program, member },
            OwnerTarget::Program(bound),
        ) if program == bound => {
            check_program(program)?;
            if let CoopMemberCandidate::Claimed {
                school,
                membership_statement,
                ..
            } = member
            {
                check_school(school)?;
                membership_statement.read(evidence)?;
            }
            Ok(())
        }
        (RelationshipCandidate::NonSchool(_), OwnerTarget::School(school)) => check_school(school),
        (RelationshipCandidate::NonSchool(_), OwnerTarget::Program(program)) => {
            check_program(program)
        }
        _ => Err(AffiliationCandidateError::RelationshipOwnerMismatch),
    }
}

fn check_school(school: &SchoolId) -> Result<(), AffiliationCandidateError> {
    if !school.validate("sch") || school.as_str().len() > 512 {
        return Err(AffiliationCandidateError::InvalidExistingId);
    }
    Ok(())
}

fn check_program(program: &TeamId) -> Result<(), AffiliationCandidateError> {
    if !program.validate("team") || program.as_str().len() > 512 {
        return Err(AffiliationCandidateError::InvalidExistingId);
    }
    Ok(())
}
