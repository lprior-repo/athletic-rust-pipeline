use std::num::NonZeroU64;
use std::path::{Component, Path, PathBuf};

use super::MemoryEnvelopeError;
use files::{decimal, with_bytes, FileRole};

mod files;

pub(super) fn resolve_current_scope(
    proc_cgroup: &Path,
    mount: &Path,
) -> Result<PathBuf, MemoryEnvelopeError> {
    with_bytes(proc_cgroup, FileRole::Membership, |bytes| {
        let malformed = || MemoryEnvelopeError::MembershipMalformed {
            path: proc_cgroup.into(),
        };
        let text = std::str::from_utf8(bytes).map_err(|_| malformed())?;
        let mut memberships = text.lines().filter_map(|line| line.strip_prefix("0::"));
        let relative = memberships.next().ok_or_else(malformed)?;
        if memberships.next().is_some() {
            return Err(malformed());
        }
        resolve_scope(mount, relative).ok_or_else(malformed)
    })
}

pub(super) fn resolve_scope(mount: &Path, relative: &str) -> Option<PathBuf> {
    if !mount.is_absolute()
        || !relative.starts_with('/')
        || relative.chars().any(char::is_control)
        || relative.split('/').any(|part| matches!(part, "." | ".."))
    {
        return None;
    }
    let mut out = mount.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::RootDir => {}
            Component::Normal(part) => out.push(part),
            _ => return None,
        }
    }
    Some(out)
}

pub(super) fn read_hard_limit(
    scope: &Path,
    budget: NonZeroU64,
) -> Result<NonZeroU64, MemoryEnvelopeError> {
    let path = scope.join("memory.max");
    with_bytes(&path, FileRole::Counter, |raw| {
        if raw.trim_ascii() == b"max" {
            return Err(MemoryEnvelopeError::UnboundedLimit { path: path.clone() });
        }
        let limit = decimal(raw)
            .and_then(NonZeroU64::new)
            .ok_or_else(|| MemoryEnvelopeError::CounterMalformed { path: path.clone() })?;
        if limit > budget {
            return Err(MemoryEnvelopeError::HardLimitAboveBudget {
                hard_limit: limit.get(),
                budget: budget.get(),
            });
        }
        Ok(limit)
    })
}

pub(super) fn read_swap(scope: &Path) -> Result<(), MemoryEnvelopeError> {
    let path = scope.join("memory.swap.max");
    with_bytes(&path, FileRole::Counter, |raw| match decimal(raw) {
        Some(0) => Ok(()),
        Some(_) => Err(MemoryEnvelopeError::NonZeroSwap {
            path: path.clone(),
            actual: String::from_utf8_lossy(raw.trim_ascii()).into_owned(),
        }),
        None if raw.trim_ascii() == b"max" => Err(MemoryEnvelopeError::NonZeroSwap {
            path: path.clone(),
            actual: "max".into(),
        }),
        None => Err(MemoryEnvelopeError::CounterMalformed { path: path.clone() }),
    })
}

fn counter(scope: &Path, name: &str) -> Result<u64, MemoryEnvelopeError> {
    let path = scope.join(name);
    with_bytes(&path, FileRole::Counter, |raw| {
        decimal(raw).ok_or_else(|| MemoryEnvelopeError::CounterMalformed { path: path.clone() })
    })
}

pub(super) fn read_usage(
    scope: &Path,
    hard_limit: NonZeroU64,
) -> Result<(u64, u64), MemoryEnvelopeError> {
    let current = counter(scope, "memory.current")?;
    let peak = counter(scope, "memory.peak")?;
    if current > peak {
        return Err(MemoryEnvelopeError::InconsistentUsage {
            scope: scope.into(),
            current,
            peak,
        });
    }
    if current > hard_limit.get() || peak > hard_limit.get() {
        return Err(MemoryEnvelopeError::LimitExceeded {
            current,
            peak,
            hard_limit: hard_limit.get(),
        });
    }
    Ok((current, peak))
}
