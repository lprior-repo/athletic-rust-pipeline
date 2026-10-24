use super::selected_proofs::objects;
use crate::report::{ReportError, ReportResult};
use std::path::{Path, PathBuf};

pub(super) struct InitialObjects(State);

enum State {
    Fresh,
    CheckedRecovery {
        directory: PathBuf,
        initial: Vec<objects::Reference>,
    },
}

impl InitialObjects {
    pub(super) fn fresh() -> Self {
        Self(State::Fresh)
    }

    pub(super) fn checked_recovery(directory: &Path) -> ReportResult<Self> {
        let initial = objects::initial_inventory(directory)?;
        let policy = Self(State::CheckedRecovery {
            directory: directory.to_path_buf(),
            initial,
        });
        policy.require_initial(directory)?;
        Ok(policy)
    }

    pub(super) fn require_initial(&self, directory: &Path) -> ReportResult<()> {
        verify(directory, self.references(directory)?)
    }

    pub(super) fn require_with_roots(
        &self,
        directory: &Path,
        roots: &[objects::Reference],
    ) -> ReportResult<()> {
        let initial = self.references(directory)?;
        if initial.len() > objects::MAX_OBJECTS || roots.len() > objects::MAX_OBJECTS {
            return Err(invalid(
                "initial proof union source exceeds its count bound",
            ));
        }
        let count = initial
            .len()
            .checked_add(roots.len())
            .ok_or_else(|| invalid("initial proof union count overflow"))?;
        let mut expected = Vec::new();
        expected
            .try_reserve(count)
            .map_err(|_| invalid("initial proof union allocation refused"))?;
        expected.extend_from_slice(initial);
        expected.extend_from_slice(roots);
        expected.sort_unstable_by(|a, b| a.digest.cmp(&b.digest));
        expected.dedup();
        verify(directory, &expected)
    }

    fn references(&self, directory: &Path) -> ReportResult<&[objects::Reference]> {
        match &self.0 {
            State::Fresh => Ok(&[]),
            State::CheckedRecovery {
                directory: captured,
                initial,
            } if captured == directory => Ok(initial),
            State::CheckedRecovery { .. } => {
                Err(invalid("initial proof directory binding mismatch"))
            }
        }
    }
}

fn verify(directory: &Path, expected: &[objects::Reference]) -> ReportResult<()> {
    if expected.is_empty() {
        objects::require_empty(directory)
    } else {
        objects::verify(directory, expected)
    }
}

fn invalid(detail: &str) -> ReportError {
    ReportError::Invariant {
        detail: detail.to_owned(),
    }
}

#[cfg(test)]
mod tests;
