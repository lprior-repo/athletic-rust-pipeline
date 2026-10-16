use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreIdentityError {
    #[error("store identity must contain exactly 64 ASCII hexadecimal bytes")]
    InvalidFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(try_from = "String")]
pub struct StoreIdentity(String);

impl StoreIdentity {
    pub fn try_clone(&self) -> Result<Self, std::collections::TryReserveError> {
        let mut value = String::new();
        value.try_reserve_exact(self.0.len())?;
        value.push_str(&self.0);
        Ok(Self(value))
    }
    pub fn parse(value: &str) -> Result<Self, StoreIdentityError> {
        Self::validate(value)?;
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(value: &str) -> Result<(), StoreIdentityError> {
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(StoreIdentityError::InvalidFormat);
        }
        Ok(())
    }
}

impl Serialize for StoreIdentity {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl TryFrom<String> for StoreIdentity {
    type Error = StoreIdentityError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::validate(&value)?;
        Ok(Self(value))
    }
}

impl From<StoreIdentity> for String {
    fn from(value: StoreIdentity) -> Self {
        value.0
    }
}
