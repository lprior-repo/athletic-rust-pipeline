use base64::{engine::general_purpose::STANDARD, Engine};
use census_domain::model::{
    CanonicalAthlete, CanonicalPerformance, RelayResult, ReviewCase, ReviewVerdictRecord,
    SourceObservation,
};

use super::{
    invalid, reserve, wire, AuthorityTableV2, DigestV2, TableBindingV2, MAX_RAW_KEY_BASE64_BYTES,
    MAX_RAW_KEY_BYTES, MAX_RAW_VALUE_BYTES, RAW_FRAGMENT_BYTES,
};
use crate::{StoreError, StoreResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawPhysicalRowV2 {
    binding: DigestV2,
    key: Vec<u8>,
    value: Vec<u8>,
}

impl RawPhysicalRowV2 {
    pub fn new(binding: &TableBindingV2, key: &[u8], value: &[u8]) -> StoreResult<Self> {
        check_lengths(key.len(), value.len())?;
        validate_physical(binding, key, value)?;
        let mut owned_key = reserve(key.len())?;
        owned_key.extend_from_slice(key);
        let mut owned_value = reserve(value.len())?;
        owned_value.extend_from_slice(value);
        Ok(Self {
            binding: binding.digest()?,
            key: owned_key,
            value: owned_value,
        })
    }

    pub fn physical_key(&self) -> &[u8] {
        &self.key
    }
    pub fn raw_value(&self) -> &[u8] {
        &self.value
    }

    pub(super) fn raw_bytes(&self) -> StoreResult<u32> {
        u32::try_from(
            self.key
                .len()
                .checked_add(self.value.len())
                .ok_or(StoreError::CounterOverflow)?,
        )
        .map_err(|_| StoreError::CounterOverflow)
    }

    pub(super) fn matches_binding(&self, binding: &DigestV2) -> bool {
        self.binding == *binding
    }

    pub(super) fn header(&self) -> StoreResult<wire::RawRow> {
        Ok(wire::RawRow {
            physical_key_base64: STANDARD.encode(&self.key),
            value_bytes: u32::try_from(self.value.len())
                .map_err(|_| StoreError::CounterOverflow)?,
            value_sha256: DigestV2::of_bytes(&self.value),
            parts: u8::try_from(self.value.chunks(RAW_FRAGMENT_BYTES).len())
                .map_err(|_| StoreError::CounterOverflow)?,
        })
    }

    pub(super) fn from_parts(
        binding: &TableBindingV2,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> StoreResult<Self> {
        check_lengths(key.len(), value.len())?;
        validate_physical(binding, &key, &value)?;
        Ok(Self {
            binding: binding.digest()?,
            key,
            value,
        })
    }
}

pub(super) fn check_lengths(key: usize, value: usize) -> StoreResult<()> {
    if key == 0 || key > MAX_RAW_KEY_BYTES || value == 0 || value > MAX_RAW_VALUE_BYTES {
        return Err(invalid("raw key or value length"));
    }
    Ok(())
}

pub(super) fn decode_base64(text: &str, maximum: usize) -> StoreResult<Vec<u8>> {
    let encoded_maximum = maximum
        .checked_add(2)
        .and_then(|n| n.checked_div(3))
        .and_then(|n| n.checked_mul(4))
        .ok_or(StoreError::CounterOverflow)?;
    if text.len() > encoded_maximum {
        return Err(invalid("base64 encoded bound"));
    }
    let decoded = STANDARD
        .decode(text)
        .map_err(|_| invalid("base64 decoding"))?;
    if decoded.len() > maximum || STANDARD.encode(&decoded) != text {
        return Err(invalid("noncanonical base64 or decoded bound"));
    }
    Ok(decoded)
}

pub(super) fn decode_key(text: &str) -> StoreResult<Vec<u8>> {
    if text.len() > MAX_RAW_KEY_BASE64_BYTES {
        return Err(invalid("raw key base64 bound"));
    }
    decode_base64(text, MAX_RAW_KEY_BYTES)
}

fn validate_physical(binding: &TableBindingV2, key: &[u8], value: &[u8]) -> StoreResult<()> {
    let table = binding.table().table();
    match binding.0.layout {
        wire::Layout::DerivedGeneration { generation } => {
            let (actual, _) = crate::keys::view_derived_key(table, key)
                .ok_or_else(|| invalid("derived physical key"))?;
            if actual != generation {
                return Err(invalid("derived generation mismatch"));
            }
        }
        wire::Layout::DerivedMap => {
            let (_, _, sequence) = crate::keys::view_observation_key(key)
                .ok_or_else(|| invalid("derived map physical key"))?;
            if sequence != 0 {
                return Err(invalid("derived map sequence must be zero"));
            }
        }
        wire::Layout::ObservationLog => {}
    }
    match binding.table() {
        AuthorityTableV2::Athletes => {
            crate::decode_physical_observation::<CanonicalAthlete>(table, key, value).map(drop)
        }
        AuthorityTableV2::SourceObservations => {
            crate::decode_physical_observation::<SourceObservation>(table, key, value).map(drop)
        }
        AuthorityTableV2::ReviewCases => {
            crate::decode_physical_observation::<ReviewCase>(table, key, value).map(drop)
        }
        AuthorityTableV2::IdentityVerdicts => {
            crate::decode_physical_observation::<ReviewVerdictRecord>(table, key, value).map(drop)
        }
        AuthorityTableV2::Performances => {
            crate::decode_physical_observation::<CanonicalPerformance>(table, key, value).map(drop)
        }
        AuthorityTableV2::RelayResults => {
            crate::decode_physical_observation::<RelayResult>(table, key, value).map(drop)
        }
    }
}
