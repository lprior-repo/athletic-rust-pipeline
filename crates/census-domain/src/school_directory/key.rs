use crate::UsJurisdiction;
use serde::{Deserialize, Serialize};

use super::address::CityName;
use super::ids::{NcesSchoolId, PssId, StateRecordId};
use super::name::{MatchForm, SchoolName};
use super::DirectoryError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct WeakKey {
    name: MatchForm,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    city: Option<MatchForm>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    state: Option<UsJurisdiction>,
}

impl WeakKey {
    pub fn of(
        name: &SchoolName,
        city: Option<&CityName>,
        state: Option<UsJurisdiction>,
    ) -> Result<Self, DirectoryError> {
        let form = MatchForm::of(name.as_str());
        if form.is_empty() {
            return Err(DirectoryError::EmptyField {
                field: "matching name",
            });
        }
        let city = city.map(|city| MatchForm::of(city.as_str()));
        if city.as_ref().is_some_and(MatchForm::is_empty) {
            return Err(DirectoryError::EmptyField {
                field: "matching city",
            });
        }
        Ok(Self {
            name: form,
            city,
            state,
        })
    }

    pub fn name(&self) -> &MatchForm {
        &self.name
    }

    pub fn city(&self) -> Option<&MatchForm> {
        self.city.as_ref()
    }

    pub fn state(&self) -> Option<UsJurisdiction> {
        self.state
    }

    pub fn label(&self) -> String {
        let city = self.city.as_ref().map(MatchForm::as_str).unwrap_or("");
        let state = self.state.map(UsJurisdiction::code).unwrap_or("");
        format!("{}|{city}|{state}", self.name.as_str())
    }
}

impl Serialize for WeakKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Self::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for WeakKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let key = Self::deserialize(deserializer)?;
        key.validate().map_err(serde::de::Error::custom)?;
        Ok(key)
    }
}

impl WeakKey {
    fn validate(&self) -> Result<(), DirectoryError> {
        if self.name.is_empty() {
            return Err(DirectoryError::EmptyField {
                field: "matching name",
            });
        }
        if self.city.as_ref().is_some_and(MatchForm::is_empty) {
            return Err(DirectoryError::EmptyField {
                field: "matching city",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DirectoryKey {
    Nces(NcesSchoolId),
    Pss(PssId),
    StateRecord {
        state: UsJurisdiction,
        id: StateRecordId,
    },
    Weak(WeakKey),
}

impl DirectoryKey {
    pub const fn rank(&self) -> u8 {
        match self {
            Self::Nces(_) => 0,
            Self::Pss(_) => 1,
            Self::StateRecord { .. } => 2,
            Self::Weak(_) => 3,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Nces(id) => format!("nces:{}", id.as_str()),
            Self::Pss(id) => format!("pss:{}", id.as_str()),
            Self::StateRecord { state, id } => {
                format!("state:{}:{}", state.code(), id.as_str())
            }
            Self::Weak(key) => format!("weak:{}", key.label()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum IdentifiedKey {
    Nces(NcesSchoolId),
    Pss(PssId),
    StateRecord {
        state: UsJurisdiction,
        id: StateRecordId,
    },
}

impl From<IdentifiedKey> for DirectoryKey {
    fn from(key: IdentifiedKey) -> Self {
        match key {
            IdentifiedKey::Nces(id) => Self::Nces(id),
            IdentifiedKey::Pss(id) => Self::Pss(id),
            IdentifiedKey::StateRecord { state, id } => Self::StateRecord { state, id },
        }
    }
}
