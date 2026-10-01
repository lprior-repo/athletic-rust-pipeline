use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("credential {name} is not set in the environment")]
pub struct MissingCredential {
    pub name: &'static str,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretKey(String);

impl SecretKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn from_env(name: &'static str) -> Result<Self, MissingCredential> {
        match std::env::var(name) {
            Ok(value) if !value.trim().is_empty() => Ok(Self(value)),
            _ => Err(MissingCredential { name }),
        }
    }

    pub fn from_env_first(names: &[&'static str]) -> Result<Self, MissingCredential> {
        for name in names {
            if let Ok(key) = Self::from_env(name) {
                return Ok(key);
            }
        }
        Err(MissingCredential {
            name: names.first().copied().unwrap_or("credential"),
        })
    }

    pub(crate) fn expose(&self) -> &str {
        self.0.as_str()
    }

    pub(crate) fn redact(&self, text: &str) -> String {
        if self.0.is_empty() {
            return text.to_string();
        }
        text.replace(self.0.as_str(), "<redacted>")
    }
}

impl fmt::Debug for SecretKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretKey(<redacted>)")
    }
}
