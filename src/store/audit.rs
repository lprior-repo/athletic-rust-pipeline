use super::{
    error::map_database_error,
    keys::{digest_for, document_key, valid_document_size},
    ArtifactStore, Result, StoreError,
};
use crate::domain::identity::EvidenceDigest;
use fjall::PersistMode;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use uuid::Uuid;

// Evidence only: no retry count, due time, ownership, lease, or executable work is stored here.
// Restate's Rust SDK does not expose receipts from failed run attempts. Publication is atomic;
// a crash before publication can leave an uncertain external effect, never an exactly-once claim.
const MAX_ATTEMPT_EVIDENCE: usize = 1_024;

#[derive(Serialize, Deserialize)]
struct StoredAttempt<T> {
    operation: EvidenceDigest,
    attempt_id: Uuid,
    value: T,
}

#[derive(Debug)]
pub struct AttemptEvidence<T> {
    pub digest: EvidenceDigest,
    pub value: T,
}

impl ArtifactStore {
    pub fn record_attempt<T: Serialize>(
        &self,
        operation: &EvidenceDigest,
        value: &T,
    ) -> Result<EvidenceDigest> {
        let id = Uuid::now_v7();
        let bytes = serde_json::to_vec(&StoredAttempt {
            operation: operation.clone(),
            attempt_id: id,
            value,
        })?;
        valid_document_size(&bytes)?;
        let digest = digest_for(&bytes)?;
        let mut key = attempt_prefix(operation);
        key.extend_from_slice(id.as_bytes());
        let _writer = self.inner.writer.lock().map_err(|_| StoreError::Database)?;
        if self
            .inner
            .attempts
            .contains_key(&key)
            .map_err(map_database_error)?
        {
            return Err(StoreError::CorruptData);
        }
        let mut batch = self.inner.database.batch();
        batch.insert(&self.inner.documents, document_key(&digest), bytes);
        batch.insert(&self.inner.attempts, key, digest.as_str().as_bytes());
        batch
            .durability(Some(PersistMode::SyncAll))
            .commit()
            .map_err(map_database_error)?;
        Ok(digest)
    }

    pub fn attempt_digests(&self, operation: &EvidenceDigest) -> Result<Vec<EvidenceDigest>> {
        let mut digests = Vec::new();
        // One past the cap so the visitor rejects the overflow; the constant cannot saturate.
        self.inner
            .attempts
            .prefix(attempt_prefix(operation))
            .take(MAX_ATTEMPT_EVIDENCE.saturating_add(1))
            .try_for_each(|guard| {
                if digests.len() == MAX_ATTEMPT_EVIDENCE {
                    return Err(StoreError::BatchTooLarge);
                }
                let (_, value) = guard.into_inner().map_err(map_database_error)?;
                let text = std::str::from_utf8(&value).map_err(|_| StoreError::CorruptData)?;
                let digest = EvidenceDigest::parse(text).map_err(|_| StoreError::CorruptData)?;
                digests.push(digest);
                Ok(())
            })?;
        Ok(digests)
    }

    pub fn read_attempt<T: DeserializeOwned>(
        &self,
        operation: &EvidenceDigest,
        digest: &EvidenceDigest,
    ) -> Result<AttemptEvidence<T>> {
        let bytes = self.get_bytes(digest)?;
        let record: StoredAttempt<T> = serde_json::from_slice(&bytes)?;
        if record.operation != *operation {
            return Err(StoreError::CorruptData);
        }
        let mut key = attempt_prefix(operation);
        key.extend_from_slice(record.attempt_id.as_bytes());
        let index = self
            .inner
            .attempts
            .get(key)
            .map_err(map_database_error)?
            .ok_or(StoreError::CorruptData)?;
        if index.as_ref() != digest.as_str().as_bytes() {
            return Err(StoreError::CorruptData);
        }
        Ok(AttemptEvidence {
            digest: digest.clone(),
            value: record.value,
        })
    }
}

fn attempt_prefix(operation: &EvidenceDigest) -> Vec<u8> {
    let mut prefix = Vec::with_capacity(81);
    prefix.extend_from_slice(operation.as_str().as_bytes());
    prefix.push(b'/');
    prefix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_effects_remain_distinct_durable_and_operation_bound() -> anyhow::Result<()> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("artifacts");
        let operation = EvidenceDigest::parse(&"a".repeat(64))?;
        let other = EvidenceDigest::parse(&"b".repeat(64))?;
        let receipt = serde_json::json!({"status": 503, "retry_after_ms": 1_000});
        let store = ArtifactStore::open(&root)?;
        let first = store.record_attempt(&operation, &receipt)?;
        let second = store.record_attempt(&operation, &receipt)?;
        anyhow::ensure!(
            first != second,
            "left={:?} right={:?} must differ",
            &first,
            &second
        );
        drop(store);
        let reopened = ArtifactStore::open(&root)?;
        let mut retained = reopened.attempt_digests(&operation)?;
        retained.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        let mut expected = vec![first.clone(), second.clone()];
        expected.sort_by(|left, right| left.as_str().cmp(right.as_str()));
        anyhow::ensure!(
            retained == expected,
            "left={:?} right={:?}",
            &retained,
            &expected
        );
        {
            let left_value = &(reopened
                .read_attempt::<serde_json::Value>(&operation, &first)?
                .value);
            let right_value = &receipt;
            anyhow::ensure!(
                left_value == right_value,
                "left={left_value:?} right={right_value:?}"
            );
        }
        {
            let left_value = &(reopened
                .read_attempt::<serde_json::Value>(&operation, &second)?
                .value);
            let right_value = &receipt;
            anyhow::ensure!(
                left_value == right_value,
                "left={left_value:?} right={right_value:?}"
            );
        }
        anyhow::ensure!(matches!(
            reopened.read_attempt::<serde_json::Value>(&other, &first),
            Err(StoreError::CorruptData)
        ));
        anyhow::ensure!(reopened.attempt_digests(&other)?.is_empty());
        Ok(())
    }
}
