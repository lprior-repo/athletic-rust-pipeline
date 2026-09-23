//! Canonical model for the independent Midwest HS TF/XC recruiting graph.
//!
//! Identity rules enforced here:
//!
//! * Every entity has a **locally minted, deterministic opaque id** derived from its natural key.
//!   No external vendor id is ever the canonical identity; vendor ids live in
//!   [`SourceIdentity`] lists and can disappear without invalidating a canonical record.
//! * Cohort membership is [`GradYear`] — an absolute, immutable property. Grade level is never a
//!   cohort key; it is recorded as [`ObservedGrade`] together with the school year and source that
//!   observed it, and only *deterministically implies* a [`GradYear`].
//! * Events are described by our own ontology ([`EventKind`]); vendor event names are source evidence
//!   ([`SourceEventLabel`]), not canonical keys.

use crate::jurisdiction::UsJurisdiction;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;

mod collision;
mod natural_key;
mod records;
mod review;

pub use collision::{id_collision, CANONICAL_ID_COLLISION_FAMILY};
pub use natural_key::NaturalKey;

pub use records::{
    AccessBlockKind, CanonicalMerge, CollectionSnapshot, CoverageRow, CoverageScope,
    RetainedConflict, ReviewCase, ReviewState, SourceAccessCondition, SourceAthleteObservation,
    SourceEntityKind, SourceMeetRef, SourceObjectIdentity, SourceObservation,
    SourceSchoolObservation, ATHLETE_IDENTITY_FAMILY, COHORT_DECISION_FAMILIES,
    COHORT_EVIDENCE_FAMILY, COHORT_IDENTITY_CONFIDENCE_FAMILY, COHORT_UNVERIFIED_FAMILY,
    CONTACT_CONFLICT_FAMILY, SCHOOL_IDENTITY_FAMILY, UNRESOLVED_SCHOOL_FAMILY,
    UNRESOLVED_VENUE_FAMILY, WITHHELD_MAILBOX_FAMILY,
};
pub use review::{
    ReviewCaseFact, ReviewEvidenceFact, ReviewPacket, ReviewVerdict, ReviewVerdictKind,
    ReviewVerdictRecord, VerdictBatch,
};

mod athlete;
mod classification;
mod coach;
mod cohort;
mod contact;
mod event_ontology;
mod event_performance;
mod identifiers;
mod meet;
mod normalization;
mod provenance;
mod school;

pub use athlete::CanonicalAthlete;
pub use classification::{CanonicalTeam, CompetitionLevel, Gender, Sport};
pub use coach::{CanonicalCoach, CoachRole};
pub use cohort::{GradYear, Grade, ObservedGrade, SchoolYear};
pub use contact::{professional_email, CONSUMER_MAIL_DOMAINS};
pub use event_ontology::{EventKind, SourceEventLabel};
pub use event_performance::{CanonicalEvent, CanonicalPerformance, Mark, TimingMethod};
pub use identifiers::{
    tag, AthleteId, CoachId, EventId, Id, MeetId, PerformanceId, SchoolId, TeamId,
};
pub use meet::{CanonicalMeet, MEET_STATE_UNRESOLVED};
pub use normalization::{flip_last_first, normalize_name, Counters};
pub use provenance::{
    Confidence, Evidence, EvidenceMethod, SourceIdentity, SourceNamespace, SourceRef,
};
pub use school::CanonicalSchool;

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
