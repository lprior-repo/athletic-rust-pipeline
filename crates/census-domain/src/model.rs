use crate::jurisdiction::UsJurisdiction;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;

mod athlete;
mod canonical_json;
mod classification;
mod coach;
mod cohort;
mod collision;
mod contact;
mod contact_proof;
mod contact_tenure;
mod dates;
mod event_ontology;
mod event_performance;
mod fixed_mark;
mod identifiers;
mod identity_aliases;
mod identity_application;
mod identity_corroboration;
mod identity_decision;
mod identity_index;
mod identity_projection;
mod identity_validation;
mod meet;
mod natural_key;
mod normalization;
mod provenance;
mod records;
mod review;
mod run;
mod school;
mod school_address;

pub use athlete::{AthleteCandidateKey, CanonicalAthlete};
pub use canonical_json::{serialized_digest, CanonicalJsonError};
pub use classification::{CanonicalTeam, CompetitionLevel, Gender, Sport};
pub use coach::{CanonicalCoach, CoachRole};
pub use cohort::{GradYear, Grade, ObservedGrade, PublishedGraduation, SchoolYear};
pub use collision::{id_collision, CANONICAL_ID_COLLISION_FAMILY};
pub use contact::{is_consumer_domain, published_email, MailboxKind};
pub use contact_proof::{
    claim_binds_to_row, compute_contact_proof, verify_contact_proof, ContactClaimEvidence,
    ContactProofError, ContactProofField, RawContactRow, ValidatedContactProof, CONTACT_COLUMNS,
    CONTACT_PROOF_COLUMN,
};
pub use contact_tenure::{
    assess_coach_tenure, validate_tenure_evidence, CoachContactClaim, CoachContactProgram,
    CoachTenure, CoachTenureEvidence, TenureAssessmentError, TenureValidation,
};
pub use event_ontology::{EventKind, SourceEventLabel};
pub use event_performance::{CanonicalEvent, CanonicalPerformance, Mark, TimingMethod};
pub use fixed_mark::{CentiMetres, CentiPoints, CentiSeconds};
pub(crate) use identifiers::write_escaped;
pub use identifiers::{
    tag, AthleteCandidateId, AthleteId, AthleteIndexId, CoachId, EventId, Id, IdTag, MeetId,
    PerformanceId, SchoolId, TeamId,
};
pub use identity_application::{AcceptedAthleteIdentity, IdentityApplication};
pub use identity_decision::{
    athlete_identity_digest, identity_verdict_digest, person_key, person_provider,
    AppliedAthleteIdentity, AppliedIdentityKind, IdentityDecisionError, IdentityMember,
    ATHLETE_IDENTITY_POLICY,
};
pub use identity_index::{AthleteIdentityIndex, IdentityError};
pub use identity_projection::{AthleteIdentityProjection, IdentityProjectionBuilder};
pub use identity_validation::{IdentityDecisionIssue, VerdictKind};
pub use meet::{CanonicalMeet, MEET_STATE_UNRESOLVED};
pub use natural_key::NaturalKey;
pub use normalization::{flip_last_first, normalize_name};
pub use provenance::{
    Confidence, ConfidenceError, Evidence, EvidenceMethod, IdentityStatus, SourceIdentity,
    SourceNamespace, SourceRef,
};
pub use records::{
    AccessBlockKind, CaseEvidence, CollectionSnapshot, CoverageRow, CoverageScope, EvidenceFact,
    RetainedConflict, ReviewCase, ReviewState, SourceAccessCondition, SourceAthleteObservation,
    SourceEntityKind, SourceMeetRef, SourceObjectIdentity, SourceObservation,
    SourceSchoolObservation, ATHLETE_IDENTITY_FAMILY, COHORT_DECISION_FAMILIES,
    COHORT_EVIDENCE_FAMILY, COHORT_UNVERIFIED_FAMILY, CONTACT_CONFLICT_FAMILY,
    IDENTITY_UNVERIFIED_FAMILY, MEMBER_SET_LABEL, SCHOOL_IDENTITY_FAMILY, UNRESOLVED_SCHOOL_FAMILY,
    UNRESOLVED_VENUE_FAMILY, UNSUPPORTED_GRADUATION_FAMILY,
};
pub use review::{
    ReviewCaseFact, ReviewEvidenceFact, ReviewPacket, ReviewVerdict, ReviewVerdictKind,
    ReviewVerdictRecord, VerdictBatch,
};
pub use run::{CensusRun, RunManifest};
pub use school::CanonicalSchool;
pub use school_address::{SchoolAddressError, SchoolPostalAddress};

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
