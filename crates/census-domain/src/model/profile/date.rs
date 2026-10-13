use super::ProfileError;
use chrono::{Datelike, NaiveDate};
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProfileDate {
    date: NaiveDate,
    iso: String,
}

impl ProfileDate {
    pub fn parse(value: &str) -> Result<Self, ProfileError> {
        if value.len() != 10
            || !value.bytes().enumerate().all(|(index, byte)| match index {
                4 | 7 => byte == b'-',
                _ => byte.is_ascii_digit(),
            })
        {
            return Err(ProfileError::InvalidDate);
        }
        let date =
            NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| ProfileError::InvalidDate)?;
        if !(1900..=2100).contains(&date.year()) {
            return Err(ProfileError::InvalidDate);
        }
        let mut iso = String::new();
        iso.try_reserve_exact(10)
            .map_err(|_| ProfileError::AllocationFailed)?;
        iso.push_str(value);
        Ok(Self { date, iso })
    }

    pub fn as_str(&self) -> &str {
        &self.iso
    }
}

impl Serialize for ProfileDate {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

struct DateVisitor;

impl Visitor<'_> for DateVisitor {
    type Value = ProfileDate;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a real YYYY-MM-DD date in 1900 through 2100")
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        ProfileDate::parse(value).map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for ProfileDate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(DateVisitor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProfileDateRangeV1 {
    from: ProfileDate,
    through: ProfileDate,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RangeInput {
    from: ProfileDate,
    through: ProfileDate,
}

impl ProfileDateRangeV1 {
    pub fn new(from: ProfileDate, through: ProfileDate) -> Result<Self, ProfileError> {
        if from > through {
            return Err(ProfileError::ReversedDateRange);
        }
        Ok(Self { from, through })
    }

    pub fn from(&self) -> &ProfileDate {
        &self.from
    }

    pub fn through(&self) -> &ProfileDate {
        &self.through
    }

    pub fn contains(&self, date: &ProfileDate) -> bool {
        &self.from <= date && date <= &self.through
    }
}

impl<'de> Deserialize<'de> for ProfileDateRangeV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let input = RangeInput::deserialize(deserializer)?;
        Self::new(input.from, input.through).map_err(serde::de::Error::custom)
    }
}
