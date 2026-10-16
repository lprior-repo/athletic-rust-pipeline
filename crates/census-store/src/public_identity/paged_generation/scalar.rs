use census_domain::model::serialized_digest;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::invalid;
use crate::{StoreResult, Table};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DigestV2(String);

impl DigestV2 {
    pub fn parse(value: &str) -> StoreResult<Self> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid("SHA-256 must be 64 lowercase hexadecimal bytes"));
        }
        Ok(Self(value.to_owned()))
    }

    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(format!("{:x}", Sha256::digest(bytes)))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(super) fn of_serialized<T: Serialize>(value: &T) -> StoreResult<Self> {
        serialized_digest(value)
            .map(Self)
            .map_err(|_| invalid("binding digest encoding"))
    }
}

impl TryFrom<String> for DigestV2 {
    type Error = crate::StoreError;
    fn try_from(value: String) -> StoreResult<Self> {
        Self::parse(&value)
    }
}

impl From<DigestV2> for String {
    fn from(value: DigestV2) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityTableV2 {
    Athletes,
    SourceObservations,
    ReviewCases,
    IdentityVerdicts,
    Performances,
    RelayResults,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableRoleV2 {
    Authority,
    Affected,
}

impl AuthorityTableV2 {
    pub const ALL: [Self; 6] = [
        Self::Athletes,
        Self::SourceObservations,
        Self::ReviewCases,
        Self::IdentityVerdicts,
        Self::Performances,
        Self::RelayResults,
    ];

    pub fn role(self) -> TableRoleV2 {
        match self {
            Self::Athletes
            | Self::SourceObservations
            | Self::ReviewCases
            | Self::IdentityVerdicts => TableRoleV2::Authority,
            Self::Performances | Self::RelayResults => TableRoleV2::Affected,
        }
    }

    pub(super) fn table(self) -> Table {
        match self {
            Self::Athletes => Table::Athletes,
            Self::SourceObservations => Table::SourceObservations,
            Self::ReviewCases => Table::ReviewCases,
            Self::IdentityVerdicts => Table::IdentityVerdicts,
            Self::Performances => Table::Performances,
            Self::RelayResults => Table::RelayResults,
        }
    }

    pub(super) fn index(self) -> usize {
        match self {
            Self::Athletes => 0,
            Self::SourceObservations => 1,
            Self::ReviewCases => 2,
            Self::IdentityVerdicts => 3,
            Self::Performances => 4,
            Self::RelayResults => 5,
        }
    }
}
