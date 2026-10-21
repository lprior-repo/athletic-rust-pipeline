use super::{AcquisitionDigest, CaptureDomainError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AdmissionKind {
    LegacyRestateSourceSelectionV1,
    ProfileBoundPublicFrontierV1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StateKey {
    State,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SelectedSource {
    Milesplit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SelectedArm {
    MilesplitResults,
    MilesplitPublicResources,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DurableRole {
    StructuredWholeMeet,
    PublicResource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "AdmissionWire")]
pub struct SourceAdmissionV1 {
    kind: AdmissionKind,
    state_key: StateKey,
    plan_fingerprint: AcquisitionDigest,
    selected_source: SelectedSource,
    selected_arm: SelectedArm,
    durable_role: DurableRole,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AdmissionWire {
    kind: AdmissionKind,
    state_key: StateKey,
    plan_fingerprint: AcquisitionDigest,
    selected_source: SelectedSource,
    selected_arm: SelectedArm,
    durable_role: DurableRole,
}
impl SourceAdmissionV1 {
    pub fn try_clone(&self) -> Result<Self, std::collections::TryReserveError> {
        Ok(Self {
            kind: self.kind,
            state_key: self.state_key,
            plan_fingerprint: self.plan_fingerprint.try_clone()?,
            selected_source: self.selected_source,
            selected_arm: self.selected_arm,
            durable_role: self.durable_role,
        })
    }
    pub fn milesplit_results(plan_fingerprint: &str) -> Result<Self, CaptureDomainError> {
        Ok(Self {
            kind: AdmissionKind::LegacyRestateSourceSelectionV1,
            state_key: StateKey::State,
            plan_fingerprint: AcquisitionDigest::parse(plan_fingerprint)?,
            selected_source: SelectedSource::Milesplit,
            selected_arm: SelectedArm::MilesplitResults,
            durable_role: DurableRole::StructuredWholeMeet,
        })
    }
    pub fn plan_fingerprint(&self) -> &AcquisitionDigest {
        &self.plan_fingerprint
    }
    pub fn public_resources(plan_fingerprint: &str) -> Result<Self, CaptureDomainError> {
        Ok(Self {
            kind: AdmissionKind::ProfileBoundPublicFrontierV1,
            state_key: StateKey::State,
            plan_fingerprint: AcquisitionDigest::parse(plan_fingerprint)?,
            selected_source: SelectedSource::Milesplit,
            selected_arm: SelectedArm::MilesplitPublicResources,
            durable_role: DurableRole::PublicResource,
        })
    }
    pub fn is_public_resources(&self) -> bool {
        self.kind == AdmissionKind::ProfileBoundPublicFrontierV1
    }
}
impl TryFrom<AdmissionWire> for SourceAdmissionV1 {
    type Error = CaptureDomainError;
    fn try_from(wire: AdmissionWire) -> Result<Self, Self::Error> {
        let (StateKey::State, SelectedSource::Milesplit) = (wire.state_key, wire.selected_source);
        match (wire.kind, wire.selected_arm, wire.durable_role) {
            (
                AdmissionKind::LegacyRestateSourceSelectionV1,
                SelectedArm::MilesplitResults,
                DurableRole::StructuredWholeMeet,
            ) => Self::milesplit_results(wire.plan_fingerprint.as_str()),
            (
                AdmissionKind::ProfileBoundPublicFrontierV1,
                SelectedArm::MilesplitPublicResources,
                DurableRole::PublicResource,
            ) => Self::public_resources(wire.plan_fingerprint.as_str()),
            _ => Err(CaptureDomainError::Invalid {
                field: "source admission role",
            }),
        }
    }
}
