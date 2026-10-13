use super::{ProfileError, MAX_PROFILE_CATEGORIES, MAX_PROFILE_GRADUATION_YEARS};
use super::{MAX_PROFILE_JURISDICTIONS, MAX_PROFILE_SPORTS};
use crate::jurisdiction::UsJurisdiction;
use crate::model::{Gender, GradYear, Sport};
use serde::de::{DeserializeSeed, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSetField {
    GraduationYears,
    Jurisdictions,
    Sports,
    CompetitionCategories,
}

impl ProfileSetField {
    pub fn maximum(self) -> usize {
        match self {
            Self::GraduationYears => MAX_PROFILE_GRADUATION_YEARS,
            Self::Jurisdictions => MAX_PROFILE_JURISDICTIONS,
            Self::Sports => MAX_PROFILE_SPORTS,
            Self::CompetitionCategories => MAX_PROFILE_CATEGORIES,
        }
    }
}

pub(super) fn normalize<T: Ord>(
    values: &mut Vec<T>,
    field: ProfileSetField,
) -> Result<(), ProfileError> {
    if values.is_empty() {
        return Err(ProfileError::EmptySet { field });
    }
    if values.len() > field.maximum() {
        return Err(ProfileError::TooManyElements {
            field,
            observed: values.len(),
            max: field.maximum(),
        });
    }
    values.sort_unstable();
    values.dedup();
    Ok(())
}

struct BoundedSetVisitor<T> {
    field: ProfileSetField,
    marker: PhantomData<T>,
}

impl<'de, T: Deserialize<'de>> Visitor<'de> for BoundedSetVisitor<T> {
    type Value = Vec<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "at most {} {:?} elements",
            self.field.maximum(),
            self.field
        )
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
        let maximum = self.field.maximum();
        if sequence.size_hint().is_some_and(|size| size > maximum) {
            return Err(serde::de::Error::custom(
                "profile container exceeds element ceiling",
            ));
        }
        let mut values = Vec::new();
        values
            .try_reserve_exact(maximum)
            .map_err(|_| serde::de::Error::custom(ProfileError::AllocationFailed))?;
        for _ in 0..maximum {
            match sequence.next_element()? {
                Some(value) => values.push(value),
                None => return Ok(values),
            }
        }
        if sequence.next_element_seed(ExcessElement)?.is_some() {
            return Err(serde::de::Error::custom(
                "profile container exceeds element ceiling",
            ));
        }
        Ok(values)
    }
}

fn bounded<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
    field: ProfileSetField,
) -> Result<Vec<T>, D::Error> {
    deserializer.deserialize_seq(BoundedSetVisitor {
        field,
        marker: PhantomData,
    })
}

pub(super) fn graduation_years<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<GradYear>, D::Error> {
    bounded(deserializer, ProfileSetField::GraduationYears)
}

pub(super) fn jurisdictions<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<UsJurisdiction>, D::Error> {
    bounded(deserializer, ProfileSetField::Jurisdictions)
}

pub(super) fn sports<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<Sport>, D::Error> {
    bounded(deserializer, ProfileSetField::Sports)
}

pub(super) fn categories<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Gender>, D::Error> {
    bounded(deserializer, ProfileSetField::CompetitionCategories)
}

struct ExcessElement;

impl<'de> DeserializeSeed<'de> for ExcessElement {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, _deserializer: D) -> Result<Self::Value, D::Error> {
        Err(serde::de::Error::custom(
            "profile container exceeds element ceiling",
        ))
    }
}
