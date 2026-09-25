//! Derived rows the census keeps beside its canonical entities: the retained conflict and review
//! queues, the source-to-canonical joins, what a source observed, the merges it decided, and the
//! measurements of a pass.
//!
//! These are [values](crate::model), not tables. They carry no clock, no store handle and no JSON
//! value, so the domain crate stays pure and the store decides how a row is keyed and durable. Every
//! id here is a deterministic function of the row's own facts, which is what lets the same finding be
//! re-derived on a later run without minting a second case.
//!
//! The rows are grouped by what they are evidence of, one module each: [`source`] holds the §31
//! `source object -> canonical row` join and the enumerated meets, [`observation`] holds what a source
//! published about a school or an athlete before any canonical decision,
//! [`source_observation`] carries that pair as one row a store can key, [`merge`] holds the merges
//! this program decided so one can be reversed, and [`measure`] holds coverage, snapshots and access
//! conditions. The queues this module keeps itself are the two findings a pass acts on: the conflict
//! the merge retained, and the review case the lane asks about.

mod evidence;
mod measure;
mod merge;
mod observation;
mod review_record;
mod source;
mod source_observation;

pub use evidence::{CaseEvidence, EvidenceFact, MEMBER_SET_LABEL};
pub use measure::{
    AccessBlockKind, CollectionSnapshot, CoverageRow, CoverageScope, SourceAccessCondition,
};
pub use merge::CanonicalMerge;
pub use observation::{SourceAthleteObservation, SourceSchoolObservation};
pub use review_record::{
    RetainedConflict, ReviewCase, ReviewState, ATHLETE_IDENTITY_FAMILY, COHORT_DECISION_FAMILIES,
    COHORT_EVIDENCE_FAMILY, COHORT_IDENTITY_CONFIDENCE_FAMILY, COHORT_UNVERIFIED_FAMILY,
    CONTACT_CONFLICT_FAMILY, SCHOOL_IDENTITY_FAMILY, UNRESOLVED_SCHOOL_FAMILY,
    UNRESOLVED_VENUE_FAMILY,
};
pub use source::{SourceEntityKind, SourceMeetRef, SourceObjectIdentity};
pub use source_observation::SourceObservation;

#[cfg(test)]
#[path = "records_tests.rs"]
mod tests;
