use super::MAX_REGION_CONTRACT_RECORD_BYTES;
use super::{keys, read, wire, RegionContractRegistration, RegionContractRegistryError as Error};
use crate::{Store, StoreError, StoreResult};
use census_domain::model::{ContractRefV1, RegionMembershipContractV1};
use fjall::PersistMode;

impl Store {
    pub fn register_region_contract(
        &self,
        reference: &ContractRefV1,
        payload: &RegionMembershipContractV1,
    ) -> StoreResult<RegionContractRegistration> {
        let computed = payload.reference().map_err(Error::InvalidPayload)?;
        if &computed != reference {
            return Err(Error::ImmutableConflict {
                requested: reference.clone(),
                computed,
            }
            .into());
        }
        let bytes = wire::encode_payload(payload)?;
        let _appends = self.lock_appends();
        if let Some(held) = read::retained(&self.db.snapshot(), &self.meta, reference)? {
            if &held.payload != payload || held.bytes != bytes {
                return Err(Error::NoncanonicalPayload {
                    reference: reference.clone(),
                }
                .into());
            }
            return Ok(RegionContractRegistration::Repeated(reference.clone()));
        }
        self.insert_region_contract(reference, &bytes)?;
        Ok(RegionContractRegistration::Registered(reference.clone()))
    }

    fn insert_region_contract(&self, reference: &ContractRefV1, bytes: &[u8]) -> StoreResult<()> {
        let key = keys::descriptor(reference)?;
        let descriptor = wire::Descriptor {
            version: 1,
            reference: reference.clone(),
            encoding: "canonical_json".to_string(),
            payload_bytes: bytes.len(),
            chunk_bytes: MAX_REGION_CONTRACT_RECORD_BYTES,
            chunks: bytes.len().div_ceil(MAX_REGION_CONTRACT_RECORD_BYTES),
        };
        let raw = crate::records::codec::encode_canonical(&descriptor)?;
        let mut batch = self.db.batch();
        batch.insert(&self.meta, key.as_bytes(), raw);
        for (index, chunk) in bytes.chunks(MAX_REGION_CONTRACT_RECORD_BYTES).enumerate() {
            batch.insert(&self.meta, keys::chunk(&key, index)?, chunk);
        }
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })
    }
}
