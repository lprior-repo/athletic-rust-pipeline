use super::{
    CandidateQuery, LocallyCheckedAssertion, LocallyCheckedChange, SubjectGap, TemporalGap,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateOutcome<'a> {
    SingleCandidate(&'a LocallyCheckedAssertion<'a>),
    MultipleCompatibleCandidates,
    Conflicted,
    Unknown,
    SubjectUnresolved,
    EndedWithoutReplacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    Candidate,
    NotVisibleAtKnownRevision,
    OutsideSport,
    OutsideEffectiveScope,
    ExplicitIntervalEnded,
    DifferentAthlete,
    SubjectUnresolved(SubjectGap),
    TemporalUnresolved(TemporalGap),
    CoopMemberUnknown,
    WithdrawnClaim,
    RelationshipEnded,
    ChangeApplied,
    ChangeOutsideEffectiveScope,
    ChangeTargetNotVisible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactDisposition<'a> {
    Assertion {
        fact: &'a LocallyCheckedAssertion<'a>,
        disposition: Disposition,
    },
    Change {
        fact: &'a LocallyCheckedChange<'a>,
        disposition: Disposition,
    },
}

#[derive(Debug)]
#[must_use]
pub struct CandidateSelection<'a> {
    pub query: CandidateQuery<'a>,
    pub outcome: CandidateOutcome<'a>,
    pub facts: Vec<FactDisposition<'a>>,
}
