use census_domain::model::{AcquisitionSlotId, AcquisitionWorkId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::restate_services) struct PlannedOwnedTarget {
    pub(in crate::restate_services) slot_id: AcquisitionSlotId,
    pub(in crate::restate_services) work_id: AcquisitionWorkId,
    pub(in crate::restate_services) selection: TargetSelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::restate_services) enum TargetSelection {
    Unregistered,
    ExistingNoPrepared,
    Prepared(PinIndex),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::restate_services) enum MeetRoute {
    NotOwned,
    Owned(TargetIndex),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub(in crate::restate_services) struct TargetIndex(u32);

impl TargetIndex {
    pub(in crate::restate_services) fn from_position(
        value: usize,
    ) -> Result<Self, super::ResultsPlanError> {
        let value =
            u32::try_from(value).map_err(|_| super::ResultsPlanError::Invalid("target index"))?;
        Self::try_from(value)
    }

    pub(in crate::restate_services) fn position(self) -> Result<usize, super::ResultsPlanError> {
        usize::try_from(self.0).map_err(|_| super::ResultsPlanError::Invalid("target index"))
    }
}

impl TryFrom<u32> for TargetIndex {
    type Error = super::ResultsPlanError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value >= 65_536 {
            return Err(Self::Error::Invalid("target index"));
        }
        Ok(Self(value))
    }
}

impl From<TargetIndex> for u32 {
    fn from(value: TargetIndex) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub(in crate::restate_services) struct PinIndex(u8);

impl PinIndex {
    pub(in crate::restate_services) fn from_position(
        value: usize,
    ) -> Result<Self, super::ResultsPlanError> {
        let value =
            u8::try_from(value).map_err(|_| super::ResultsPlanError::Invalid("pin index"))?;
        Self::try_from(value)
    }

    pub(in crate::restate_services) fn position(self) -> usize {
        usize::from(self.0)
    }
}

impl TryFrom<u8> for PinIndex {
    type Error = super::ResultsPlanError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value >= 12 {
            return Err(Self::Error::Invalid("pin index"));
        }
        Ok(Self(value))
    }
}

impl From<PinIndex> for u8 {
    fn from(value: PinIndex) -> Self {
        value.0
    }
}
