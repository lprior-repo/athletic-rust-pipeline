use super::InstallationDomainError;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ServiceResourceProfileV1 {
    max_concurrent: usize,
    memory_budget_bytes: u64,
    drain_timeout_seconds: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    max_concurrent: usize,
    memory_budget_bytes: u64,
    drain_timeout_seconds: u64,
}

impl ServiceResourceProfileV1 {
    pub const MAX_CONCURRENT: usize = 1024;
    pub const GIB: u64 = 1024 * 1024 * 1024;
    pub const MAX_MEMORY_BYTES: u64 = 1024 * Self::GIB;

    pub fn new(
        max_concurrent: usize,
        memory_budget_bytes: u64,
        drain_timeout_seconds: u64,
    ) -> Result<Self, InstallationDomainError> {
        if !(1..=Self::MAX_CONCURRENT).contains(&max_concurrent) {
            return Err(InstallationDomainError::InvalidServiceResource {
                field: "max_concurrent",
            });
        }
        if !(Self::GIB..=Self::MAX_MEMORY_BYTES).contains(&memory_budget_bytes)
            || !memory_budget_bytes.is_multiple_of(Self::GIB)
        {
            return Err(InstallationDomainError::InvalidServiceResource {
                field: "memory_budget_bytes",
            });
        }
        Ok(Self {
            max_concurrent,
            memory_budget_bytes,
            drain_timeout_seconds,
        })
    }

    pub const fn max_concurrent(self) -> usize {
        self.max_concurrent
    }
    pub const fn memory_budget_bytes(self) -> u64 {
        self.memory_budget_bytes
    }
    pub const fn drain_timeout_seconds(self) -> u64 {
        self.drain_timeout_seconds
    }
}

impl<'de> Deserialize<'de> for ServiceResourceProfileV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let input = Input::deserialize(deserializer)?;
        Self::new(
            input.max_concurrent,
            input.memory_budget_bytes,
            input.drain_timeout_seconds,
        )
        .map_err(serde::de::Error::custom)
    }
}
