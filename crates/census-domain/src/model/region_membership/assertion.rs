use super::RegionSubjectV1;
use super::{ProfileDateRangeV1, RegionContextV1, RegionEvidenceV1};
use serde::{Deserialize, Deserializer, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RegionStatementV1 {
    Member,
    ExplicitNonMember,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RegionEffectiveV1 {
    Inclusive(ProfileDateRangeV1),
    Undated,
}

impl Ord for RegionEffectiveV1 {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Inclusive(left), Self::Inclusive(right)) => {
                (left.from(), left.through()).cmp(&(right.from(), right.through()))
            }
            (Self::Inclusive(_), Self::Undated) => Ordering::Less,
            (Self::Undated, Self::Inclusive(_)) => Ordering::Greater,
            (Self::Undated, Self::Undated) => Ordering::Equal,
        }
    }
}

impl PartialOrd for RegionEffectiveV1 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionAssertionInputV1 {
    pub subject: RegionSubjectV1,
    pub context: RegionContextV1,
    pub effective: RegionEffectiveV1,
    pub statement: RegionStatementV1,
    pub provenance: RegionEvidenceV1,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct RegionAssertionV1(RegionAssertionInputV1);

impl RegionAssertionV1 {
    pub fn new(input: RegionAssertionInputV1) -> Self {
        Self(input)
    }

    pub fn input(&self) -> &RegionAssertionInputV1 {
        &self.0
    }
}

impl<'de> Deserialize<'de> for RegionAssertionV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::new(RegionAssertionInputV1::deserialize(
            deserializer,
        )?))
    }
}
