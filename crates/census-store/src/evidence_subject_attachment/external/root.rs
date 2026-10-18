use census_domain::evidence_subject_attachment::AttachmentObjectReferenceV1;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetainedCaptureRootSchemaV1 {
    ParticipationPacketV1,
    ParticipationWindowPacketV1,
    ProviderTargetPacketV1,
    HtmlResultCaptureV1,
    ProviderSchoolPacketV1,
}

impl RetainedCaptureRootSchemaV1 {
    pub fn phase(self) -> &'static str {
        match self {
            Self::ParticipationPacketV1 => "independent_participation_capture_v1",
            Self::ParticipationWindowPacketV1 => "independent_participation_window_capture_v1",
            Self::ProviderTargetPacketV1 => "provider_target_intake_v1",
            Self::HtmlResultCaptureV1 => "retained_html_result_capture_v1",
            Self::ProviderSchoolPacketV1 => "provider_school_intake_v1",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedCaptureRootV1 {
    pub schema: RetainedCaptureRootSchemaV1,
    pub key: String,
    pub payload: AttachmentObjectReferenceV1,
}
