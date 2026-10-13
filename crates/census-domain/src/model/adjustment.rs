use crate::model::{Mark, SourceRef};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjustmentKind {
    TrackSizeConversion,
}

impl AdjustmentKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::TrackSizeConversion => "track_size_conversion",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjustmentBasis {
    Known,
    Unknown,
}

impl AdjustmentBasis {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Known => "known",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Adjustment {
    pub kind: AdjustmentKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_mark: Option<Mark>,
    pub adjusted_mark: Mark,
    pub basis: AdjustmentBasis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceRef>,
}

impl Adjustment {
    pub fn track_size_conversion(
        original: Option<Mark>,
        adjusted: Mark,
        note: Option<String>,
        source: Option<SourceRef>,
    ) -> Self {
        Self {
            kind: AdjustmentKind::TrackSizeConversion,
            basis: match original {
                Some(_) => AdjustmentBasis::Known,
                None => AdjustmentBasis::Unknown,
            },
            original_mark: original,
            adjusted_mark: adjusted,
            note,
            source,
        }
    }
}

