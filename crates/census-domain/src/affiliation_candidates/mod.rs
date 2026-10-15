#![forbid(unsafe_code)]
#![deny(unused_must_use)]

mod bounds;
mod changes;
pub mod current;
mod equality;
mod evidence;
mod material_work;
mod observed;
mod outcome;
mod relationship;
mod selection;
mod selection_checks;
mod selection_reduce;
mod temporal;

pub use bounds::{AffiliationCandidateError, KnownBounds, ObjectRef, Resource, SelectionGrant};
pub use changes::{ChangeKind, LocallyCheckedChange, RawChange};
pub use evidence::{FinalUrl, LocallyCheckedEvidence, RawEvidence, SourceSpan};
pub use observed::{
    CoopMemberCandidate, LocallyCheckedAssertion, NonSchoolKind, ObservedOwner, OwnerTarget,
    PersonCandidate, RawAssertion, RawOwnership, RelationshipCandidate, SubjectGap,
};
pub use outcome::{CandidateOutcome, CandidateSelection, Disposition, FactDisposition};
pub use selection::{select_candidates, CandidateQuery, DeclaredFrozenView};
pub use temporal::{CalendarCandidate, SportScope, TemporalGap, TemporalScope};

#[cfg(test)]
mod fixture;

#[cfg(test)]
mod tests;
