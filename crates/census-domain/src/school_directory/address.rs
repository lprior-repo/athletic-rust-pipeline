use crate::UsJurisdiction;
use serde::{Deserialize, Serialize};
use std::fmt;

use super::text::{bounded, presentation_case};
use super::DirectoryError;

#[path = "address_wire.rs"]
mod wire;

pub const STREET_LIMIT: usize = 120;
pub const CITY_LIMIT: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ZipCode {
    code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    plus4: Option<String>,
}

impl ZipCode {
    pub fn parse(raw: &str) -> Result<Self, DirectoryError> {
        let trimmed = raw.trim();
        let mut segments = trimmed.split('-');
        let first = segments
            .next()
            .map_or(Default::default(), core::convert::identity);
        let second = segments.next();
        if segments.next().is_some() {
            return Err(malformed(raw));
        }
        match second {
            Some(extension) => Self::of(first, Some(extension)),
            None => match first.chars().count() {
                5 => Self::of(first, None),
                9 => {
                    let code: String = first.chars().take(5).collect();
                    let extension: String = first.chars().skip(5).collect();
                    Self::of(&code, Some(&extension))
                }
                _ => Err(malformed(raw)),
            },
        }
    }

    pub fn of(code: &str, plus4: Option<&str>) -> Result<Self, DirectoryError> {
        if !is_digits(code.trim(), 5) {
            return Err(malformed(code));
        }
        let plus4 = match plus4 {
            None => None,
            Some(raw) => {
                let extension = raw.trim();
                if extension.is_empty() {
                    None
                } else if is_digits(extension, 4) {
                    Some(extension.to_string())
                } else {
                    return Err(malformed(&format!("{code}-{extension}")));
                }
            }
        };
        Ok(Self {
            code: code.trim().to_string(),
            plus4,
        })
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn plus4(&self) -> Option<&str> {
        self.plus4.as_deref()
    }
}

impl fmt::Display for ZipCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.code)?;
        if let Some(plus4) = &self.plus4 {
            formatter.write_str("-")?;
            formatter.write_str(plus4)?;
        }
        Ok(())
    }
}

fn malformed(value: &str) -> DirectoryError {
    DirectoryError::MalformedZip {
        value: value.to_string(),
    }
}

fn is_digits(value: &str, length: usize) -> bool {
    value.chars().count() == length && value.chars().all(|ch| ch.is_ascii_digit())
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct StreetLine(String);

impl StreetLine {
    pub fn parse(raw: &str) -> Result<Self, DirectoryError> {
        Ok(Self(bounded(
            raw,
            "street",
            STREET_LIMIT,
            presentation_case,
        )?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct CityName(String);

impl CityName {
    pub fn parse(raw: &str) -> Result<Self, DirectoryError> {
        Ok(Self(bounded(raw, "city", CITY_LIMIT, presentation_case)?))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum AddressKind {
    Physical,
    Mailing,
    #[default]
    Unknown,
}

impl AddressKind {
    fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct PostalAddress {
    #[serde(default, skip_serializing_if = "AddressKind::is_unknown")]
    kind: AddressKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    line1: Option<StreetLine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    line2: Option<StreetLine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    city: Option<CityName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    state: Option<UsJurisdiction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    zip: Option<ZipCode>,
}

impl PostalAddress {
    pub fn of(
        line1: Option<StreetLine>,
        line2: Option<StreetLine>,
        city: Option<CityName>,
        state: Option<UsJurisdiction>,
        zip: Option<ZipCode>,
    ) -> Option<Self> {
        let address = Self {
            kind: AddressKind::Unknown,
            line1,
            line2,
            city,
            state,
            zip,
        };
        if address.is_empty() {
            None
        } else {
            Some(address)
        }
    }

    pub fn line(line1: StreetLine) -> Self {
        Self {
            kind: AddressKind::Unknown,
            line1: Some(line1),
            line2: None,
            city: None,
            state: None,
            zip: None,
        }
    }

    pub fn kind(&self) -> AddressKind {
        self.kind
    }

    pub fn with_kind(mut self, kind: AddressKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn is_empty(&self) -> bool {
        self.line1.is_none()
            && self.line2.is_none()
            && self.city.is_none()
            && self.state.is_none()
            && self.zip.is_none()
    }

    pub fn line1(&self) -> Option<&StreetLine> {
        self.line1.as_ref()
    }

    pub fn line2(&self) -> Option<&StreetLine> {
        self.line2.as_ref()
    }

    pub fn city(&self) -> Option<&CityName> {
        self.city.as_ref()
    }

    pub fn state(&self) -> Option<UsJurisdiction> {
        self.state
    }

    pub fn zip(&self) -> Option<&ZipCode> {
        self.zip.as_ref()
    }

    pub fn with_city(mut self, city: CityName) -> Self {
        self.city = Some(city);
        self
    }

    pub fn with_state(mut self, state: UsJurisdiction) -> Self {
        self.state = Some(state);
        self
    }

    pub fn with_zip(mut self, zip: ZipCode) -> Self {
        self.zip = Some(zip);
        self
    }
}
