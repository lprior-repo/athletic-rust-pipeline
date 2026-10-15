use super::{Credit, Receipt, Store, StoreError, StoreResult};
use crate::meta;
use fjall::PersistMode;

const CREDIT_PREFIX: &str = "endpoint_credit/";

fn credit_key(endpoint: &str) -> String {
    format!("{CREDIT_PREFIX}{endpoint}")
}

impl Store {
    pub fn credit_observations(&self, endpoint: &str, operation: &str) -> StoreResult<Credit> {
        crate::public_identity::refuse_reserved(operation)?;
        if operation.starts_with("source_acquisition_attempt_") {
            return Err(crate::CaptureError::CaptureQualificationOptInRequired.into());
        }
        if endpoint.is_empty() {
            return Err(StoreError::Refused {
                detail: "crediting an unnamed endpoint would total into nothing".to_string(),
            });
        }
        let _appends = self.lock_appends();
        let receipt = self
            .receipt(operation)?
            .ok_or_else(|| StoreError::Refused {
                detail: format!(
                "operation {operation} has no receipt: observations are credited only after they \
                 are applied"
            ),
            })?;
        let key = credit_key(endpoint);
        let standing = meta::get_u64(&self.meta, &key)?.map_or(0, core::convert::identity);
        if receipt.credited {
            return Ok(Credit {
                credited: 0,
                total: standing,
            });
        }
        let credited = receipt.appended;
        let total = standing
            .checked_add(credited)
            .ok_or(StoreError::CounterOverflow)?;
        let marked = Receipt {
            credited: true,
            ..receipt
        };
        let value = serde_json::to_vec(&marked).map_err(|source| StoreError::Json {
            detail: "serializing a credited receipt".to_string(),
            source,
        })?;
        let mut batch = self.db.batch();
        batch.insert(&self.receipts, operation.as_bytes(), value);
        meta::put_text(&mut batch, &self.meta, &key, &total.to_string());
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        Ok(Credit { credited, total })
    }

    pub fn endpoint_credit(&self, endpoint: &str) -> StoreResult<u64> {
        Ok(meta::get_u64(&self.meta, &credit_key(endpoint))?.map_or(0, core::convert::identity))
    }
}
