use super::CaptureDomainError;
use serde::{Deserialize, Serialize};

pub(super) fn try_text(value: &str) -> Result<String, std::collections::TryReserveError> {
    let mut copy = String::new();
    copy.try_reserve_exact(value.len())?;
    copy.push_str(value);
    Ok(copy)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub(super) struct VersionOne;
impl TryFrom<u8> for VersionOne {
    type Error = CaptureDomainError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == 1 {
            Ok(Self)
        } else {
            Err(CaptureDomainError::Version(value))
        }
    }
}
impl From<VersionOne> for u8 {
    fn from(_: VersionOne) -> Self {
        1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub(super) struct VersionTwo;
impl TryFrom<u8> for VersionTwo {
    type Error = CaptureDomainError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == 2 {
            Ok(Self)
        } else {
            Err(CaptureDomainError::Version(value))
        }
    }
}
impl From<VersionTwo> for u8 {
    fn from(_: VersionTwo) -> Self {
        2
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(try_from = "String")]
pub struct AcquisitionDigest(String);
impl AcquisitionDigest {
    pub fn try_clone(&self) -> Result<Self, std::collections::TryReserveError> {
        try_text(&self.0).map(Self)
    }
    pub fn parse(value: &str) -> Result<Self, CaptureDomainError> {
        Self::validate(value)?;
        Ok(Self(value.to_string()))
    }
    fn validate(value: &str) -> Result<(), CaptureDomainError> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(CaptureDomainError::Invalid { field: "digest" });
        }
        Ok(())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl Serialize for AcquisitionDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}
impl TryFrom<String> for AcquisitionDigest {
    type Error = CaptureDomainError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::validate(&value)?;
        Ok(Self(value))
    }
}
impl From<AcquisitionDigest> for String {
    fn from(value: AcquisitionDigest) -> Self {
        value.0
    }
}
impl std::fmt::Display for AcquisitionDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AcquisitionSlotId(AcquisitionDigest);
impl AcquisitionSlotId {
    pub fn try_clone(&self) -> Result<Self, std::collections::TryReserveError> {
        self.0.try_clone().map(Self)
    }
    pub fn parse(value: &str) -> Result<Self, CaptureDomainError> {
        AcquisitionDigest::parse(value).map(Self)
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl std::fmt::Display for AcquisitionSlotId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AcquisitionWorkId(AcquisitionDigest);
impl AcquisitionWorkId {
    pub fn try_clone(&self) -> Result<Self, std::collections::TryReserveError> {
        self.0.try_clone().map(Self)
    }
    pub fn parse(value: &str) -> Result<Self, CaptureDomainError> {
        AcquisitionDigest::parse(value).map(Self)
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}
impl std::fmt::Display for AcquisitionWorkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct AttemptOrdinal(u8);
impl AttemptOrdinal {
    pub fn new(value: u8) -> Result<Self, CaptureDomainError> {
        if !(1..=3).contains(&value) {
            return Err(CaptureDomainError::Invalid {
                field: "attempt ordinal",
            });
        }
        Ok(Self(value))
    }
    pub fn get(self) -> u8 {
        self.0
    }
    pub fn next(self) -> Result<Self, CaptureDomainError> {
        Self::new(self.0.checked_add(1).ok_or(CaptureDomainError::Invalid {
            field: "attempt ordinal",
        })?)
    }
}
impl TryFrom<u8> for AttemptOrdinal {
    type Error = CaptureDomainError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<AttemptOrdinal> for u8 {
    fn from(value: AttemptOrdinal) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct HopOrdinal(u8);
impl HopOrdinal {
    pub fn new(value: u8) -> Result<Self, CaptureDomainError> {
        if value > 4 {
            return Err(CaptureDomainError::Invalid {
                field: "hop ordinal",
            });
        }
        Ok(Self(value))
    }
    pub fn get(self) -> u8 {
        self.0
    }
    pub fn next(self) -> Result<Self, CaptureDomainError> {
        Self::new(self.0.checked_add(1).ok_or(CaptureDomainError::Invalid {
            field: "hop ordinal",
        })?)
    }
}
impl TryFrom<u8> for HopOrdinal {
    type Error = CaptureDomainError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<HopOrdinal> for u8 {
    fn from(value: HopOrdinal) -> Self {
        value.0
    }
}

pub(super) fn bounded(
    value: &str,
    field: &'static str,
    max: usize,
) -> Result<(), CaptureDomainError> {
    if value.is_empty() || value.bytes().any(|b| b.is_ascii_control()) {
        return Err(CaptureDomainError::Invalid { field });
    }
    if value.len() > max {
        return Err(CaptureDomainError::Bound { field, max });
    }
    Ok(())
}

pub(super) fn url(value: &str) -> Result<url::Url, CaptureDomainError> {
    bounded(value, "url", 4096)?;
    let url = url::Url::parse(value).map_err(|_| CaptureDomainError::Invalid { field: "url" })?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(CaptureDomainError::Invalid { field: "url" });
    }
    Ok(url)
}
