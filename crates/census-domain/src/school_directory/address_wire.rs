use super::{is_digits, malformed, CityName, DirectoryError, PostalAddress, StreetLine, ZipCode};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for ZipCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Self::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for ZipCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Self::deserialize(deserializer)?;
        value.validate().map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}

impl ZipCode {
    fn validate(&self) -> Result<(), DirectoryError> {
        if !is_digits(&self.code, 5)
            || self
                .plus4
                .as_ref()
                .is_some_and(|extension| !is_digits(extension, 4))
        {
            return Err(malformed(&self.to_string()));
        }
        Ok(())
    }
}

impl Serialize for PostalAddress {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Self::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for PostalAddress {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Self::deserialize(deserializer)?;
        if value.is_empty() {
            return Err(serde::de::Error::custom(DirectoryError::EmptyField {
                field: "postal address",
            }));
        }
        Ok(value)
    }
}

impl TryFrom<String> for StreetLine {
    type Error = DirectoryError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let value = Self::parse(&raw)?;
        if value.as_str() != raw {
            return Err(DirectoryError::UnsupportedValue {
                field: "canonical street",
                value: raw,
            });
        }
        Ok(value)
    }
}

impl TryFrom<String> for CityName {
    type Error = DirectoryError;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        let value = Self::parse(&raw)?;
        if value.as_str() != raw {
            return Err(DirectoryError::UnsupportedValue {
                field: "canonical city",
                value: raw,
            });
        }
        Ok(value)
    }
}
