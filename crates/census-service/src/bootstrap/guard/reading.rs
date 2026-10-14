use std::num::NonZeroU64;

use super::cgroup::CgroupMemoryReader;
use crate::bootstrap::{BootstrapError, MemoryAccounting, ServeOptions};

pub(in crate::bootstrap) struct MemoryReader {
    scope: Scope,
    budget_bytes: u64,
}

enum Scope {
    ProcessRss,
    Cgroup {
        reader: CgroupMemoryReader,
        peak_bytes: u64,
    },
}

impl MemoryReader {
    pub(in crate::bootstrap) fn prepare(options: &ServeOptions) -> Result<Self, BootstrapError> {
        match &options.memory_accounting {
            MemoryAccounting::ProcessRss => {
                tracing::warn!("process RSS accounting is not a whole-deployment memory envelope");
                Ok(Self {
                    scope: Scope::ProcessRss,
                    budget_bytes: options.memory_budget_bytes,
                })
            }
            MemoryAccounting::CgroupV2 { scope } => {
                let budget = NonZeroU64::new(options.memory_budget_bytes)
                    .ok_or(BootstrapError::MemoryBudgetIsZero)?;
                let reader = CgroupMemoryReader::declared(budget, scope).map_err(unavailable)?;
                let sample = reader.sample().map_err(unavailable)?;
                tracing::info!(scope = %sample.scope.display(), current_bytes = sample.current_bytes,
                    peak_bytes = sample.peak_bytes, hard_limit_bytes = sample.hard_limit_bytes.get(),
                    "finite cgroup memory accounting admitted");
                Ok(Self {
                    scope: Scope::Cgroup {
                        reader,
                        peak_bytes: sample.peak_bytes,
                    },
                    budget_bytes: sample.hard_limit_bytes.get(),
                })
            }
        }
    }

    pub(in crate::bootstrap) fn budget_bytes(&self) -> u64 {
        self.budget_bytes
    }

    pub(super) fn read(&mut self) -> Option<u64> {
        let (reader, peak_bytes) = match &mut self.scope {
            Scope::ProcessRss => return super::resident_bytes(),
            Scope::Cgroup { reader, peak_bytes } => (reader, peak_bytes),
        };
        match reader.sample() {
            Ok(sample) if sample.hard_limit_bytes.get() == self.budget_bytes => {
                if sample.peak_bytes < *peak_bytes {
                    tracing::error!(previous_peak_bytes = *peak_bytes, observed_peak_bytes = sample.peak_bytes,
                        current_bytes = sample.current_bytes, scope = %sample.scope.display(),
                        "admitted cgroup high-water accounting regressed");
                    return None;
                }
                *peak_bytes = sample.peak_bytes;
                tracing::debug!(scope = %sample.scope.display(), current_bytes = sample.current_bytes,
                    peak_bytes = sample.peak_bytes, "cgroup memory sample");
                Some(sample.current_bytes)
            }
            Ok(sample) => {
                tracing::error!(expected = self.budget_bytes, observed = sample.hard_limit_bytes.get(),
                    scope = %sample.scope.display(), "admitted cgroup hard limit changed");
                None
            }
            Err(error) => {
                tracing::error!(%error, "admitted cgroup memory accounting failed");
                None
            }
        }
    }
}

fn unavailable(error: impl std::fmt::Display) -> BootstrapError {
    BootstrapError::MemoryAccountingUnavailable {
        detail: error.to_string(),
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests;
