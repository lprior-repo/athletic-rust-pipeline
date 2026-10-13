use super::{serialized_digest, CanonicalJsonError, Gender, GradYear, SchoolYear, Sport};
use crate::jurisdiction::UsJurisdiction;
use serde::{Deserialize, Deserializer, Serialize};

mod date;
mod references;
mod selection;
mod sets;

#[cfg(test)]
mod tests;

pub use date::{ProfileDate, ProfileDateRangeV1};
pub use references::{ContractRefV1, ProfileId, ProfilePoliciesV1};
pub use selection::{ProfileSelectionError, ProfileSelectionV1};
pub use sets::ProfileSetField;

pub const QUERY_PROFILE_VERSION: u32 = 1;
pub const MAX_PROFILE_GRADUATION_YEARS: usize = 64;
pub const MAX_PROFILE_JURISDICTIONS: usize = 51;
pub const MAX_PROFILE_SPORTS: usize = 3;
pub const MAX_PROFILE_CATEGORIES: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProfileError {
    #[error("unsupported query-profile version {found}")]
    UnsupportedVersion { found: u32 },
    #[error("{field:?} must be an explicit nonempty set")]
    EmptySet { field: ProfileSetField },
    #[error("{field:?} contains {observed} elements, exceeding {max}")]
    TooManyElements {
        field: ProfileSetField,
        observed: usize,
        max: usize,
    },
    #[error("unknown sport is not an admitted TF/XC sport")]
    UnknownSport,
    #[error("unknown category is not an explicit boys/girls/mixed category")]
    UnknownCategory,
    #[error("profile date must be a real ISO day in 1900 through 2100")]
    InvalidDate,
    #[error("profile date range must have from <= through")]
    ReversedDateRange,
    #[error("performance-window through must not exceed as_of")]
    PerformanceAfterAsOf,
    #[error("digest must be exactly 64 lowercase hexadecimal bytes")]
    InvalidDigest,
    #[error("unsupported immutable-contract reference version {found}")]
    UnsupportedContractVersion { found: u32 },
    #[error("immutable-contract revision must be nonzero")]
    ZeroContractRevision,
    #[error("profile boundary allocation failed")]
    AllocationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RegionalMembershipV1 {
    AllSelectedJurisdictions,
    SourceBackedRegion(ContractRefV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum DiscoveryPriorityV1 {
    Uniform,
    SourceBackedRegionFirst(ContractRefV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ProvisionalInclusionV1 {
    Exclude,
    Include,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum PerformanceWindowV1 {
    AllRetainedThroughAsOf,
    Inclusive(ProfileDateRangeV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryProfileInputV1 {
    pub version: u32,
    #[serde(deserialize_with = "sets::graduation_years")]
    pub graduation_years: Vec<GradYear>,
    #[serde(deserialize_with = "sets::jurisdictions")]
    pub jurisdictions: Vec<UsJurisdiction>,
    pub regional_membership: RegionalMembershipV1,
    pub discovery_priority: DiscoveryPriorityV1,
    #[serde(deserialize_with = "sets::sports")]
    pub sports: Vec<Sport>,
    #[serde(deserialize_with = "sets::categories")]
    pub competition_categories: Vec<Gender>,
    pub as_of: ProfileDate,
    pub performance_window: PerformanceWindowV1,
    pub contact_school_year: SchoolYear,
    pub source_plan: ContractRefV1,
    pub acquisition_window: ProfileDateRangeV1,
    pub completion_contract: ContractRefV1,
    pub policies: ProfilePoliciesV1,
    pub include_provisional: ProvisionalInclusionV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct QueryProfileV1(QueryProfileInputV1);

impl QueryProfileV1 {
    pub fn new(mut input: QueryProfileInputV1) -> Result<Self, ProfileError> {
        if input.version != QUERY_PROFILE_VERSION {
            return Err(ProfileError::UnsupportedVersion {
                found: input.version,
            });
        }
        sets::normalize(
            &mut input.graduation_years,
            ProfileSetField::GraduationYears,
        )?;
        sets::normalize(&mut input.jurisdictions, ProfileSetField::Jurisdictions)?;
        sets::normalize(&mut input.sports, ProfileSetField::Sports)?;
        sets::normalize(
            &mut input.competition_categories,
            ProfileSetField::CompetitionCategories,
        )?;
        if input.sports.contains(&Sport::Unknown) {
            return Err(ProfileError::UnknownSport);
        }
        if input.competition_categories.contains(&Gender::Unknown) {
            return Err(ProfileError::UnknownCategory);
        }
        if let PerformanceWindowV1::Inclusive(range) = &input.performance_window {
            if range.through() > &input.as_of {
                return Err(ProfileError::PerformanceAfterAsOf);
            }
        }
        Ok(Self(input))
    }

    pub fn id(&self) -> Result<ProfileId, CanonicalJsonError> {
        let digest = serialized_digest(self)?;
        ProfileId::parse(&digest)
            .map_err(|error| CanonicalJsonError::Unsupported(error.to_string()))
    }

    pub fn input(&self) -> &QueryProfileInputV1 {
        &self.0
    }

    pub fn admits_jurisdiction(&self, jurisdiction: UsJurisdiction) -> bool {
        self.0.jurisdictions.binary_search(&jurisdiction).is_ok()
    }

    pub fn admits_performance_date(&self, date: ProfileDate) -> bool {
        if date > self.0.as_of {
            return false;
        }
        match &self.0.performance_window {
            PerformanceWindowV1::AllRetainedThroughAsOf => true,
            PerformanceWindowV1::Inclusive(range) => range.contains(&date),
        }
    }
}

impl<'de> Deserialize<'de> for QueryProfileV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(QueryProfileInputV1::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
