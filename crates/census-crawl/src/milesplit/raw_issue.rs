use super::SourceRowLocator;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawGradeIssueKind {
    OutsideHighSchool,
    Unrecognized,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawGradeIssue {
    pub row: SourceRowLocator,
    pub raw_token: String,
    pub kind: RawGradeIssueKind,
}
