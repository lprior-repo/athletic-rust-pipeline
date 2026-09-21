//! Geographic fact composition: which of city and region the source actually
//! carries, and when two locations are compatible.

use super::names::{CityName, RegionName};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Location {
    Missing,
    CityOnly(CityName),
    RegionOnly(RegionName),
    CityRegion { city: CityName, region: RegionName },
}
impl Location {
    pub(crate) fn fields(&self) -> (Option<&CityName>, Option<&RegionName>) {
        match self {
            Self::Missing => (None, None),
            Self::CityOnly(city) => (Some(city), None),
            Self::RegionOnly(region) => (None, Some(region)),
            Self::CityRegion { city, region } => (Some(city), Some(region)),
        }
    }

    pub(crate) fn compatible_with(&self, other: &Self) -> bool {
        let (city, region) = self.fields();
        let (other_city, other_region) = other.fields();
        let city_compatible = match (city, other_city) {
            (Some(left), Some(right)) => left == right,
            _ => true,
        };
        let region_compatible = match (region, other_region) {
            (Some(left), Some(right)) => left == right,
            _ => true,
        };
        city_compatible && region_compatible
    }
}
