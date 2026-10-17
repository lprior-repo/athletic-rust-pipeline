use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub(in crate::export::dataset) enum Version {
    HistoricalContextV1,
    RetainedGraphV2,
    ReviewedRosterGraphV3,
}

impl TryFrom<u32> for Version {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::HistoricalContextV1),
            2 => Ok(Self::RetainedGraphV2),
            3 => Ok(Self::ReviewedRosterGraphV3),
            _ => Err("unsupported selected proof envelope version"),
        }
    }
}

impl From<Version> for u32 {
    fn from(value: Version) -> Self {
        match value {
            Version::HistoricalContextV1 => 1,
            Version::RetainedGraphV2 => 2,
            Version::ReviewedRosterGraphV3 => 3,
        }
    }
}
