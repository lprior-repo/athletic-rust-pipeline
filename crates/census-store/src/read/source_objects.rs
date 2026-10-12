use crate::{Store, StoreError, StoreResult};
use fjall::Readable;

impl Store {
    pub fn source_object_keys(&self, limit: usize, key_bytes: usize) -> StoreResult<Vec<String>> {
        let prefix = b"endpoint_credit/";
        let snapshot = self.db.snapshot();
        let mut keys = Vec::new();
        for guard in snapshot.prefix(&self.meta, prefix) {
            let raw = guard.key().map_err(|source| StoreError::Read { source })?;
            let suffix = raw
                .strip_prefix(prefix)
                .ok_or_else(|| StoreError::Invariant {
                    detail: "source-object key is outside the credit inventory".to_string(),
                })?;
            if keys.len() >= limit || suffix.is_empty() || suffix.len() > key_bytes {
                return Err(StoreError::Refused {
                    detail: "source-object inventory exceeds its key or count bound".to_string(),
                });
            }
            let key = std::str::from_utf8(suffix).map_err(|_| StoreError::Invariant {
                detail: "source-object credit key is not utf-8".to_string(),
            })?;
            keys.try_reserve(1).map_err(|_| StoreError::Refused {
                detail: "source-object inventory allocation refused".to_string(),
            })?;
            keys.push(key.to_string());
        }
        Ok(keys)
    }
}
