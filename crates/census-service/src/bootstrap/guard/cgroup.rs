use std::num::NonZeroU64;
use std::path::PathBuf;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux::{read_hard_limit, read_swap, read_usage, resolve_current_scope};
#[derive(Debug, thiserror::Error)]
pub(super) enum MemoryEnvelopeError {
    #[cfg(not(target_os = "linux"))]
    #[error("cgroup envelope unsupported on this platform")]
    UnsupportedPlatform,
    #[error("cgroup membership missing at {path}: {source}")]
    MembershipMissing {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cgroup membership malformed at {path}")]
    MembershipMalformed { path: PathBuf },
    #[error("cgroup membership exceeds 4096 bytes at {path}")]
    MembershipOversized { path: PathBuf },
    #[error("cgroup counter missing at {path}: {source}")]
    CounterMissing {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cgroup counter malformed at {path}")]
    CounterMalformed { path: PathBuf },
    #[error("cgroup counter exceeds 4096 bytes at {path}")]
    CounterOversized { path: PathBuf },
    #[error("cgroup memory limit is unbounded at {path}")]
    UnboundedLimit { path: PathBuf },
    #[error("cgroup swap allowance at {path} must be zero, found {actual:?}")]
    NonZeroSwap { path: PathBuf, actual: String },
    #[error("cgroup hard limit {hard_limit} bytes exceeds declared budget {budget} bytes")]
    HardLimitAboveBudget { hard_limit: u64, budget: u64 },
    #[error("cgroup scope moved from {expected} to {observed}")]
    MovedScope {
        expected: PathBuf,
        observed: PathBuf,
    },
    #[error("cgroup membership {membership} is outside declared scope {scope}")]
    ScopeNotAncestor { membership: PathBuf, scope: PathBuf },
    #[error("cgroup usage is inconsistent at {scope}: current={current}, peak={peak}")]
    InconsistentUsage {
        scope: PathBuf,
        current: u64,
        peak: u64,
    },
    #[error("cgroup usage exceeds {hard_limit} bytes: current={current}, peak={peak}")]
    LimitExceeded {
        current: u64,
        peak: u64,
        hard_limit: u64,
    },
    #[error("reading cgroup file {path} failed: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MemoryEnvelopeSample {
    pub(super) current_bytes: u64,
    pub(super) peak_bytes: u64,
    pub(super) hard_limit_bytes: NonZeroU64,
    pub(super) scope: PathBuf,
}
#[derive(Debug)]
pub(super) struct CgroupMemoryReader {
    budget: NonZeroU64,
    proc_cgroup: PathBuf,
    mount: PathBuf,
    scope: PathBuf,
    membership: PathBuf,
}
#[cfg(target_os = "linux")]
impl CgroupMemoryReader {
    pub(super) fn declared(
        budget: NonZeroU64,
        scope: &std::path::Path,
    ) -> Result<Self, MemoryEnvelopeError> {
        Self::with_declared_paths(
            budget,
            PathBuf::from("/proc/self/cgroup"),
            PathBuf::from("/sys/fs/cgroup"),
            Some(scope),
        )
    }
    #[cfg(test)]
    pub(in crate::bootstrap::guard) fn with_paths(
        budget: NonZeroU64,
        proc_cgroup: PathBuf,
        mount: PathBuf,
    ) -> Result<Self, MemoryEnvelopeError> {
        Self::with_declared_paths(budget, proc_cgroup, mount, None)
    }
    fn with_declared_paths(
        budget: NonZeroU64,
        proc_cgroup: PathBuf,
        mount: PathBuf,
        declared: Option<&std::path::Path>,
    ) -> Result<Self, MemoryEnvelopeError> {
        let membership = resolve_current_scope(&proc_cgroup, &mount)?;
        let scope = match declared {
            Some(relative) => relative
                .to_str()
                .and_then(|text| linux::resolve_scope(&mount, text))
                .ok_or_else(|| MemoryEnvelopeError::MembershipMalformed {
                    path: relative.into(),
                })?,
            None => membership.clone(),
        };
        if !membership.starts_with(&scope) {
            return Err(MemoryEnvelopeError::ScopeNotAncestor { membership, scope });
        }
        read_hard_limit(&scope, budget)?;
        read_swap(&scope)?;
        Ok(Self {
            budget,
            proc_cgroup,
            mount,
            scope,
            membership,
        })
    }
    pub(super) fn sample(&self) -> Result<MemoryEnvelopeSample, MemoryEnvelopeError> {
        let membership = resolve_current_scope(&self.proc_cgroup, &self.mount)?;
        if membership != self.membership {
            return Err(MemoryEnvelopeError::MovedScope {
                expected: self.membership.clone(),
                observed: membership,
            });
        }
        let hard = read_hard_limit(&self.scope, self.budget)?;
        read_swap(&self.scope)?;
        let (current_bytes, peak_bytes) = read_usage(&self.scope, hard)?;
        Ok(MemoryEnvelopeSample {
            current_bytes,
            peak_bytes,
            hard_limit_bytes: hard,
            scope: self.scope.clone(),
        })
    }
}
#[cfg(not(target_os = "linux"))]
impl CgroupMemoryReader {
    pub(super) fn declared(
        _budget: NonZeroU64,
        _scope: &std::path::Path,
    ) -> Result<Self, MemoryEnvelopeError> {
        Err(MemoryEnvelopeError::UnsupportedPlatform)
    }
    pub(super) fn sample(&self) -> Result<MemoryEnvelopeSample, MemoryEnvelopeError> {
        Err(MemoryEnvelopeError::UnsupportedPlatform)
    }
}
#[cfg(test)]
mod tests;
