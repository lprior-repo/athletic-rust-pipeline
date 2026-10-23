use crate::jurisdiction::UsJurisdiction;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;

mod adjustment;
mod athlete;
mod canonical_json;
mod classification;
mod coach;
mod coach_lane;
mod cohort;
mod collision;
mod combined_event;
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
mod event_semantics;
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
mod identity_review_partition;
mod identity_validation;
mod installation;
mod meet;
mod natural_key;
mod normalization;
pub mod performance_comparison;
mod performance_identity;
mod profile;
mod provenance;
mod records;
mod relay_result;
mod result_status;
mod review;
mod revision;
mod run;
mod school;
mod school_address;
mod school_contact;
mod serde_bounds;
mod store_identity;
pub use installation::{
    InstallationCapabilityV1, InstallationConfigId, InstallationConfigV1, InstallationDomainError,
    InstallationEnrollmentV1, ServiceResourceProfileV1,
};

pub use profile::{
    ContractRefV1, DiscoveryPriorityV1, PerformanceWindowV1, ProfileDate, ProfileDateRangeV1,
    ProfileError, ProfileId, ProfilePoliciesV1, ProfileSelectionError, ProfileSelectionV1,
    ProfileSetField, ProvisionalInclusionV1, QueryProfileInputV1, QueryProfileV1,
    RegionalMembershipV1, MAX_PROFILE_CATEGORIES, MAX_PROFILE_GRADUATION_YEARS,
    MAX_PROFILE_JURISDICTIONS, MAX_PROFILE_SPORTS, QUERY_PROFILE_VERSION,
};

pub use revision::Revision;
pub use store_identity::{StoreIdentity, StoreIdentityError};

pub use adjustment::{Adjustment, AdjustmentBasis, AdjustmentKind};
pub use athlete::{AthleteCandidateKey, AthleteConstructionError, CanonicalAthlete};
pub use canonical_json::{
    serialized_bytes, serialized_digest, write_serialized, CanonicalJsonError,
};
pub use classification::{genders_compatible, CanonicalTeam, CompetitionLevel, Gender, Sport};
pub use coach::{CanonicalCoach, CoachRole};
pub use coach_lane::{
    CoachLaneAcceptance, CoachLaneError, CoachLaneEvidence, CoachLaneMailbox, CoachLaneMatrix,
    CoachLaneObligation, CoachLaneRefusal, CoachLaneResolution, CoachLaneRole, CoachLaneStage,
    CoachLaneStatus, CoachLaneTenure, PendingCoachLane, COACH_LANE_ID_PREFIX,
    COACH_LANE_MAX_ARM_BYTES, COACH_LANE_MAX_FRESHNESS_DAYS,
};
pub use cohort::{GradYear, Grade, ObservedGrade, PublishedGraduation, SchoolYear};
pub use collision::{id_collision, CANONICAL_ID_COLLISION_FAMILY};
pub use combined_event::{
    CombinedComponentResult, CombinedDiscipline, CombinedEligibility, CombinedEventCompletion,
    CombinedEventError, CombinedEventSpecification, MAX_COMBINED_COMPONENTS,
};
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
pub use event_semantics::EventSemantics;
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
pub use identity_review_partition::separate_generic_identity_cases;
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
    SourceSchoolObservation, ATHLETE_GENDER_UNVERIFIED_FAMILY, ATHLETE_IDENTITY_FAMILY,
    COHORT_DECISION_FAMILIES, COHORT_EVIDENCE_FAMILY, COHORT_UNVERIFIED_FAMILY,
    CONTACT_CONFLICT_FAMILY, IDENTITY_UNVERIFIED_FAMILY, MEMBER_SET_LABEL,
    RELAY_MEMBER_NAME_FAMILY, RELAY_RESULT_SCALAR_FAMILY, SCHOOL_ADDRESS_UNVERIFIED_FAMILY,
    SCHOOL_IDENTITY_FAMILY, TRANSFER_AMBIGUOUS_FAMILY, UNRESOLVED_SCHOOL_FAMILY,
    UNRESOLVED_VENUE_FAMILY, UNSUPPORTED_GRADUATION_FAMILY,
};
pub use relay_result::{RelayMember, RelayResult};
pub use result_status::{parse_result_status, ResultStatus, ResultStatusKind};
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

mod region_membership;

pub use region_membership::{
    admit_region_payload_bytes_v1, bind_region_contract_v1, resolve_region_membership_v1,
    resolve_region_priority_v1, BoundRegionContractV1, RegionAssertionDispositionV1,
    RegionAssertionInputV1, RegionAssertionTraceV1, RegionAssertionV1,
    RegionCompositionResolutionV1, RegionCompositionV1, RegionContextV1, RegionEffectiveV1,
    RegionEvidenceInputV1, RegionEvidenceV1, RegionKeyV1, RegionMembershipContractInputV1,
    RegionMembershipContractV1, RegionMembershipError, RegionMembershipQueryV1,
    RegionMembershipResolutionV1, RegionMembershipTargetV1, RegionMembershipVerdictV1,
    RegionPriorityResolutionV1, RegionPriorityTierV1, RegionProgramV1, RegionSchoolV1,
    RegionStatementV1, RegionSubjectV1, MAX_REGION_ASSERTIONS, MAX_REGION_CONSTITUENTS,
    MAX_REGION_JURISDICTIONS, MAX_REGION_PAYLOAD_BYTES, REGION_MEMBERSHIP_CONTRACT_VERSION,
};

mod recruiting_catalog;
pub use recruiting_catalog::{
    CatalogError, EventDisposition, EventExclusionReason, EventResolutionReason,
    RecruitingCatalogV1, CATALOG_V1_MAX_AGGREGATE_LABEL_BYTES, CATALOG_V1_MAX_LABEL_BYTES,
    CATALOG_V1_MAX_SOURCE_LABELS,
};

mod history;
pub use history::{HistoryError, HistoryWindow};
mod source_acquisition;
pub use source_acquisition::*;
