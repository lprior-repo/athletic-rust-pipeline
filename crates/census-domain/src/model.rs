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
mod coach_lane;
mod cohort;
mod collision;
mod contact;
mod contact_proof;
mod contact_tenure;
mod dates;
mod engine_affiliation;
mod engine_assessment;
mod engine_capture;
mod engine_claim;
mod engine_coverage;
mod engine_identity;
mod engine_performance;
mod engine_subject;
mod engine_valid;
mod event_identity;
mod event_ontology;
mod event_performance;
mod event_specification;
mod fixed_mark;
mod identifiers;
mod identity_aliases;
mod identity_application;
mod identity_attestation;
mod identity_corroboration;
mod identity_decision;
mod identity_index;
mod identity_projection;
mod identity_validation;
mod meet;
mod natural_key;
mod normalization;
mod performance_identity;
mod provenance;
mod records;
mod relay_result;
mod review;
mod run;
mod school;
mod school_address;
mod school_contact;

pub use athlete::{AthleteCandidateKey, CanonicalAthlete};
pub use canonical_json::{
    serialized_bytes, serialized_digest, write_serialized, CanonicalJsonError,
};
pub use classification::{CanonicalTeam, CompetitionLevel, Gender, Sport};
pub use coach::{CanonicalCoach, CoachRole};
pub use coach_lane::{
    CoachLaneAcceptance, CoachLaneError, CoachLaneEvidence, CoachLaneMailbox, CoachLaneMatrix,
    CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution, CoachLaneRole, CoachLaneStage,
    CoachLaneStatus, CoachLaneTenure, PendingCoachLane, COACH_LANE_ID_PREFIX,
    COACH_LANE_MAX_ARM_BYTES, COACH_LANE_MAX_FRESHNESS_DAYS,
};
pub use cohort::{GradYear, Grade, ObservedGrade, PublishedGraduation, SchoolYear};
pub use collision::{id_collision, CANONICAL_ID_COLLISION_FAMILY};
pub use contact::{is_consumer_domain, published_email, MailboxKind};
pub use contact_proof::{
    claim_binds_to_row, compute_contact_proof, is_director, verify_contact_proof,
    ContactClaimEvidence, ContactProofError, ContactProofField, RawContactRow,
    ValidatedContactProof, CONTACT_COLUMNS, CONTACT_PROOF_COLUMN,
};
pub use contact_tenure::{
    assess_coach_tenure, validate_tenure_evidence, CoachContactClaim, CoachContactProgram,
    CoachTenure, CoachTenureEvidence, TenureAssessmentError, TenureValidation,
};
pub use engine_affiliation::{AppointmentRequest, AthleteAffiliation, CoachAppointment};
pub use engine_assessment::{CohortAssessment, CohortVerdict, COHORT_ASSESSMENT_POLICY};
pub use engine_capture::SourceCapture;
pub use engine_claim::SourceClaim;
pub use engine_coverage::{CoverageObligation, CoverageState, PendingObligation};
pub use engine_identity::refuse_identity_loss;
pub use engine_performance::{
    Corroboration, PerformanceClaim, PerformanceRequest, MAX_FAMILY_BYTES, MAX_SOURCE_KEY_BYTES,
};
pub use engine_subject::SourceSubject;
pub use engine_valid::{
    MAX_CAPTURE_BYTES, MAX_DETAIL_BYTES, MAX_ENGINE_DIGEST_BYTES, MAX_LOGICAL_ID_BYTES,
    MAX_MEMBERS_HINT, MAX_PROVIDER_ID_BYTES, MAX_URL_BYTES,
};
pub use event_identity::{EventIdentity, EventIdentityError};
pub use event_ontology::{EventKind, SourceEventLabel};
pub use event_performance::{CanonicalEvent, CanonicalPerformance, Mark, TimingMethod};
pub use event_specification::{
    CompetitionCategory, CourseIdentity, CourseMeasurement, CrossCountryContext, DistanceUnit,
    EventSpecification, HurdleSpecification, ImplementMass, IndoorTrackSpecification,
    PublishedCourseConditions, PublishedDistance, SpecificationError, TrackBanking,
};
pub use fixed_mark::{CentiMetres, CentiPoints, ExactSeconds, TimeError};
pub(crate) use identifiers::write_escaped;
pub use identifiers::{
    tag, AthleteCandidateId, AthleteId, AthleteIndexId, CoachId, EventId, Id, IdTag, MeetId,
    PerformanceId, RelayId, SchoolId, TeamId,
};
pub use identity_application::{AcceptedAthleteIdentity, IdentityApplication};
pub use identity_attestation::{AttestationError, AttestationQualification, IdentityAttestation};
pub use identity_decision::{
    athlete_identity_digest, identity_verdict_digest, person_key, person_provider,
    AppliedAthleteIdentity, AppliedIdentityKind, IdentityDecisionError, IdentityMember,
    ATHLETE_IDENTITY_POLICY,
};
pub use identity_index::{AthleteIdentityIndex, IdentityError};
pub use identity_projection::{
    AthleteIdentityProjection, IdentityAliasTransition, IdentityProjectionBuilder,
};
pub use identity_validation::{IdentityDecisionIssue, VerdictKind};
pub use meet::{CanonicalMeet, MEET_STATE_UNRESOLVED};
pub use natural_key::NaturalKey;
pub use normalization::{flip_last_first, normalize_name};
pub use performance_identity::{PerformanceError, PerformanceIdentity, PerformanceResult};
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
    IDENTITY_UNVERIFIED_FAMILY, MEMBER_SET_LABEL, RELAY_MEMBER_NAME_FAMILY,
    RELAY_RESULT_SCALAR_FAMILY, SCHOOL_IDENTITY_FAMILY, UNRESOLVED_SCHOOL_FAMILY,
    UNRESOLVED_VENUE_FAMILY, UNSUPPORTED_GRADUATION_FAMILY,
};
pub use relay_result::{RelayMember, RelayResult};
pub use review::{
    ReviewCaseFact, ReviewEvidenceFact, ReviewPacket, ReviewVerdict, ReviewVerdictKind,
    ReviewVerdictRecord, VerdictBatch,
};
pub use run::{CensusRun, RunManifest};
pub use school::CanonicalSchool;
pub use school_address::{SchoolAddressError, SchoolPostalAddress};
pub use school_contact::{
    mailbox as school_mailbox, mailbox_research as school_mailbox_research,
    research as school_contact_research, source_research as school_contact_source_research,
    ContactResearch, ContactResearchAttempt, ContactResearchOutcome, ContactResearchSubject,
    SchoolContactError, SchoolMailboxClaim, SchoolMailboxPurpose,
};

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
