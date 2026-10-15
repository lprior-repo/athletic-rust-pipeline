use std::sync::{Arc, Mutex};

use crate::receipt::{self, Application, Decision};
use crate::{Store, StoreError, StoreResult};

use super::StoreBatch;

#[derive(Clone, Copy, Debug)]
pub struct StoreRevision<'store> {
    origin: &'store Arc<Mutex<()>>,
    evidence_generation: u64,
    derived_generation: u64,
}

impl<'store> StoreRevision<'store> {
    pub(crate) fn captured(store: &'store Store, evidence: u64, derived: u64) -> Self {
        Self {
            origin: &store.appends,
            evidence_generation: evidence,
            derived_generation: derived,
        }
    }

    pub fn evidence_generation(&self) -> u64 {
        self.evidence_generation
    }

    pub fn derived_generation(&self) -> u64 {
        self.derived_generation
    }

    pub(crate) fn ensure_origin(&self, store: &Store, operation: &str) -> StoreResult<()> {
        if !Arc::ptr_eq(self.origin, &store.appends) {
            return Err(StoreError::RevisionOriginMismatch {
                operation: operation.to_string(),
            });
        }
        Ok(())
    }

    pub(crate) fn ensure_current(&self, store: &Store, operation: &str) -> StoreResult<()> {
        self.ensure_origin(store, operation)?;
        let evidence = store.evidence_generation();
        let derived = store.derived_generation();
        if self.evidence_generation != evidence || self.derived_generation != derived {
            return Err(StoreError::RevisionMoved {
                operation: operation.to_string(),
                expected_evidence: self.evidence_generation,
                actual_evidence: evidence,
                expected_derived: self.derived_generation,
                actual_derived: derived,
            });
        }
        Ok(())
    }
}

pub struct RevisionedApplication<'store> {
    application: Application,
    revision: StoreRevision<'store>,
}

impl<'store> RevisionedApplication<'store> {
    pub fn into_parts(self) -> (Application, StoreRevision<'store>) {
        (self.application, self.revision)
    }
}

impl<'store> StoreBatch<'store> {
    pub fn commit_once_at_revision(
        self,
        operation: &str,
        digest: &str,
        expected: StoreRevision<'_>,
    ) -> StoreResult<RevisionedApplication<'store>> {
        receipt::refuse_over_operation(operation, digest)?;
        let store = self.store;
        let _appends = store.lock_appends();
        expected.ensure_current(store, operation)?;
        let application = match receipt::decide(store, operation, digest)? {
            Decision::Repeat(receipt) => Application::Repeated(receipt),
            Decision::First => self
                .commit_staged(Some((operation, digest)), store.db.batch())?
                .ok_or_else(|| StoreError::Invariant {
                    detail: format!("revision-fenced operation {operation} produced no receipt"),
                })?,
        };
        let revision = StoreRevision::captured(
            store,
            store.evidence_generation(),
            store.derived_generation(),
        );
        Ok(RevisionedApplication {
            application,
            revision,
        })
    }
}

#[cfg(test)]
mod tests;
