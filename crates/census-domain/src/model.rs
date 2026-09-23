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
    SourceEntityKind, SourceMeetRef, SourceObjectIdentity, SourceSchoolObservation,
    ATHLETE_IDENTITY_FAMILY, COHORT_EVIDENCE_FAMILY, COHORT_IDENTITY_CONFIDENCE_FAMILY,
    COHORT_UNVERIFIED_FAMILY, CONTACT_CONFLICT_FAMILY, SCHOOL_IDENTITY_FAMILY,
    UNRESOLVED_SCHOOL_FAMILY, UNRESOLVED_VENUE_FAMILY, WITHHELD_MAILBOX_FAMILY,
};
pub use review::{
    ReviewCaseFact, ReviewEvidenceFact, ReviewPacket, ReviewVerdict, ReviewVerdictKind,
    ReviewVerdictRecord, VerdictBatch,
};


mod identifiers;
mod cohort;
mod provenance;
mod classification;
mod event_ontology;
mod school;
mod coach;
mod athlete;
mod meet;
mod event_performance;
mod contact;
mod normalization;

pub use identifiers::{tag, Id, SchoolId, TeamId, CoachId, AthleteId, MeetId, EventId, PerformanceId };
pub use cohort::{SchoolYear, Grade, GradYear, ObservedGrade };
pub use provenance::{SourceRef, EvidenceMethod, Evidence, SourceNamespace, SourceIdentity, Confidence };
pub use classification::{Gender, Sport, CanonicalTeam, CompetitionLevel };
pub use event_ontology::{EventKind, SourceEventLabel };
pub use school::{CanonicalSchool };
pub use coach::{CanonicalCoach, CoachRole };
pub use athlete::{CanonicalAthlete };
pub use meet::{CanonicalMeet, MEET_STATE_UNRESOLVED };
pub use event_performance::{CanonicalEvent, Mark, CanonicalPerformance, TimingMethod };
pub use contact::{CONSUMER_MAIL_DOMAINS, professional_email };
pub use normalization::{normalize_name, flip_last_first, Counters };

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
