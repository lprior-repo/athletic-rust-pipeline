use super::*;
use census_domain::evidence_subject_attachment::*;
use census_domain::model::{
    serialized_bytes, CanonicalAthlete, CanonicalSchool, SourceObservation,
};
use census_store::{
    Application, AttachmentContextObjectV1, AttachmentInspectionRequestV1,
    EvidenceSubjectAttachmentInspectionV1, EvidenceSubjectAttachmentPreparationV1,
    EvidenceSubjectAttachmentStoreError, IdentityCommitOutcomeV1, Receipt,
    RetainedCaptureLeafKindV1, RetainedCaptureLeafV1, RetainedCaptureSlotV1,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;

use crate::participation_evidence::provider_school::{
    CheckedProviderSchoolPacketV1, ProviderSchoolPacketV1,
};
use census_domain::model::SourceNamespace;
#[path = "provider_school/fixtures.rs"]
mod school_fixtures;

mod bridge;
mod context;
mod diagnostics;
mod material;
mod preservation;
mod report;
mod sources;
mod subjects;
mod workflow;

use material::*;
use sources::*;

const ARTICLE_OPERATION: &str = "genuine-reviewed-article-a-v1";
const RESULTS_OPERATION: &str = "genuine-reviewed-may15-b-v1";
const INVENTORY_DIGEST: &str = "ffe74266d46b4d0324f511a55a90a4cb9fc0d4be4b4fbdf6fff023b2ac23b2f4";
const SOURCES_DIGEST: &str = "fcbbee47cc88bd078a5c843dfb9807971128fdb05015575c647fcecf4fe55e21";
