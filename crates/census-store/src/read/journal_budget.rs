use crate::journal::OBJECT_RECORD_BYTES_LIMIT;
use crate::{StoreError, StoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JournalReadBudget {
    max_encoded_bytes: usize,
}

impl JournalReadBudget {
    pub fn new(max_encoded_bytes: usize) -> StoreResult<Self> {
        if max_encoded_bytes == 0 || max_encoded_bytes > OBJECT_RECORD_BYTES_LIMIT {
            return Err(StoreError::Refused {
                detail: format!(
                    "journal read budget must be in 1..={OBJECT_RECORD_BYTES_LIMIT} encoded bytes"
                ),
            });
        }
        Ok(Self { max_encoded_bytes })
    }

    pub fn max_encoded_bytes(self) -> usize {
        self.max_encoded_bytes
    }
}
