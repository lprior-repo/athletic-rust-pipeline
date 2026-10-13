use super::{RegionMembershipError, MAX_REGION_CONSTITUENTS};
use crate::model::{SchoolId, TeamId};
use crate::UsJurisdiction;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct RegionSchoolV1 {
    school: SchoolId,
    jurisdiction: UsJurisdiction,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SchoolInput {
    school: SchoolId,
    jurisdiction: UsJurisdiction,
}

impl RegionSchoolV1 {
    pub fn new(
        school: SchoolId,
        jurisdiction: UsJurisdiction,
    ) -> Result<Self, RegionMembershipError> {
        super::boundary::school_id(&school)?;
        Ok(Self {
            school,
            jurisdiction,
        })
    }

    pub fn school(&self) -> &SchoolId {
        &self.school
    }

    pub fn jurisdiction(&self) -> UsJurisdiction {
        self.jurisdiction
    }
}

impl<'de> Deserialize<'de> for RegionSchoolV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let input = SchoolInput::deserialize(deserializer)?;
        Self::new(input.school, input.jurisdiction).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct RegionCompositionV1(CompositionInput);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum CompositionInput {
    Standalone(RegionSchoolV1),
    CoOp(#[serde(deserialize_with = "super::boundary::constituents")] Vec<RegionSchoolV1>),
    Unknown,
}

impl RegionCompositionV1 {
    pub fn standalone(school: RegionSchoolV1) -> Self {
        Self(CompositionInput::Standalone(school))
    }

    pub fn unknown() -> Self {
        Self(CompositionInput::Unknown)
    }

    pub fn co_op(mut schools: Vec<RegionSchoolV1>) -> Result<Self, RegionMembershipError> {
        super::boundary::count("constituents", schools.len(), MAX_REGION_CONSTITUENTS)?;
        schools.sort_unstable();
        schools.dedup();
        if schools.len() < 2 {
            return Err(RegionMembershipError::InvalidField {
                field: "co_op.constituents",
            });
        }
        if schools.windows(2).any(|pair| match pair {
            [left, right] => left.school() == right.school(),
            _ => false,
        }) {
            return Err(RegionMembershipError::InvalidField {
                field: "co_op.jurisdictions",
            });
        }
        Ok(Self(CompositionInput::CoOp(schools)))
    }

    pub fn schools(&self) -> &[RegionSchoolV1] {
        match &self.0 {
            CompositionInput::Standalone(school) => std::slice::from_ref(school),
            CompositionInput::CoOp(schools) => schools,
            CompositionInput::Unknown => &[],
        }
    }
}

impl<'de> Deserialize<'de> for RegionCompositionV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match CompositionInput::deserialize(deserializer)? {
            CompositionInput::Standalone(school) => Ok(Self::standalone(school)),
            CompositionInput::CoOp(schools) => {
                Self::co_op(schools).map_err(serde::de::Error::custom)
            }
            CompositionInput::Unknown => Ok(Self::unknown()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct RegionProgramV1 {
    program: TeamId,
    composition: RegionCompositionV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProgramInput {
    program: TeamId,
    composition: RegionCompositionV1,
}

impl RegionProgramV1 {
    pub fn new(
        program: TeamId,
        composition: RegionCompositionV1,
    ) -> Result<Self, RegionMembershipError> {
        super::boundary::program_id(&program)?;
        Ok(Self {
            program,
            composition,
        })
    }

    pub fn program(&self) -> &TeamId {
        &self.program
    }

    pub fn composition(&self) -> &RegionCompositionV1 {
        &self.composition
    }
}

impl<'de> Deserialize<'de> for RegionProgramV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let input = ProgramInput::deserialize(deserializer)?;
        Self::new(input.program, input.composition).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RegionSubjectV1 {
    School(RegionSchoolV1),
    Program(RegionProgramV1),
}
