use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreIdentityError {
    #[error("store identity must contain exactly 64 ASCII hexadecimal bytes")]
    InvalidFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct StoreIdentity(String);

impl StoreIdentity {
    pub fn parse(value: &str) -> Result<Self, StoreIdentityError> {
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(StoreIdentityError::InvalidFormat);
        }
        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for StoreIdentity {
    type Error = StoreIdentityError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<StoreIdentity> for String {
    fn from(value: StoreIdentity) -> Self {
        value.0
    }
}
