mod bounded;
mod object_intake;

use crate::public_identity::{
    graph_witness::HeldGraphWitness, paged_generation::ObjectRefV2, retained::Revision,
    PublicIdentityError,
};
use crate::{StoreError, StoreResult};
use base64::{engine::general_purpose::STANDARD, Engine};
use census_domain::{model::StoreIdentity, public_identity_history::ReplayLimits};
use serde::{
    de::{SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};

pub(super) const MAX_OBJECTS: usize = 4096;
pub(super) const MAX_PHYSICAL_ROWS: usize = 4096;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Bundle {
    pub version: u32,
    #[serde(deserialize_with = "bounded::identity")]
    pub store_identity: StoreIdentity,
    pub revision: Revision,
    #[serde(deserialize_with = "bounded::declaration")]
    pub declaration: String,
    #[serde(deserialize_with = "bounded::head")]
    pub head: String,
    #[serde(deserialize_with = "bounded::history")]
    pub history: String,
    #[serde(deserialize_with = "operations")]
    pub operations: Vec<Operation>,
    #[serde(deserialize_with = "objects")]
    pub objects: Vec<Object>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Operation {
    pub sequence: u64,
    #[serde(deserialize_with = "bounded::witness")]
    pub witness: String,
    #[serde(deserialize_with = "bounded::receipt")]
    pub receipt: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "bounded::decision"
    )]
    pub decision: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Object {
    #[serde(deserialize_with = "bounded::object")]
    pub reference: ObjectRefV2,
    #[serde(deserialize_with = "physical_rows")]
    pub rows: Vec<PhysicalRow>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PhysicalRow {
    #[serde(deserialize_with = "bounded::key")]
    pub key_base64: String,
    #[serde(deserialize_with = "bounded::value")]
    pub value_base64: String,
}

impl Bundle {
    pub(super) fn bounds(&self) -> StoreResult<()> {
        if self.version != 2 {
            return Err(PublicIdentityError::Corrupt("frozen graph bundle version").into());
        }
        super::super::bound(self.declaration.len(), 65_536)?;
        super::super::bound(self.head.len(), 1024)?;
        super::super::bound(self.history.len(), crate::MAX_PUBLIC_IDENTITY_HISTORY_BYTES)?;
        super::super::bound(self.operations.len(), ReplayLimits::SMALL.records)?;
        super::super::bound(self.objects.len(), MAX_OBJECTS)?;
        self.operations.iter().try_for_each(|operation| {
            HeldGraphWitness::decode("frozen graph witness", operation.witness.as_bytes())?;
            super::super::bound(operation.receipt.len(), 65_536)?;
            if let Some(bytes) = &operation.decision {
                super::super::bound(bytes.len(), 65_536)?;
            }
            Ok::<(), StoreError>(())
        })?;
        self.objects.iter().try_fold(0usize, |total, object| {
            let total = total
                .checked_add(object.rows.len())
                .ok_or(StoreError::CounterOverflow)?;
            super::super::bound(total, MAX_PHYSICAL_ROWS)?;
            Ok::<usize, StoreError>(total)
        })?;
        Ok(())
    }
}

impl PhysicalRow {
    pub(super) fn capture(key: &[u8], value: &[u8]) -> StoreResult<Self> {
        super::super::bound(key.len(), 1024)?;
        super::super::bound(value.len(), crate::MAX_JOURNAL_VALUE_BYTES)?;
        Ok(Self {
            key_base64: STANDARD.encode(key),
            value_base64: STANDARD.encode(value),
        })
    }
    pub(super) fn decode(&self) -> StoreResult<(Vec<u8>, Vec<u8>)> {
        Ok((
            binary(&self.key_base64, 1024)?,
            binary(&self.value_base64, crate::MAX_JOURNAL_VALUE_BYTES)?,
        ))
    }
}

fn binary(value: &str, limit: usize) -> StoreResult<Vec<u8>> {
    let maximum = limit
        .checked_add(2)
        .and_then(|n| n.checked_div(3))
        .and_then(|n| n.checked_mul(4))
        .ok_or(StoreError::CounterOverflow)?;
    super::super::bound(value.len(), maximum)?;
    let bytes = STANDARD
        .decode(value)
        .map_err(|_| PublicIdentityError::Corrupt("frozen graph physical base64"))?;
    super::super::bound(bytes.len(), limit)?;
    if bytes.is_empty() || STANDARD.encode(&bytes) != value {
        return Err(PublicIdentityError::Corrupt("noncanonical frozen physical bytes").into());
    }
    Ok(bytes)
}

fn operations<'de, D: Deserializer<'de>>(parser: D) -> Result<Vec<Operation>, D::Error> {
    bounded(parser, ReplayLimits::SMALL.records, |_| 0, 0)
}
fn objects<'de, D: Deserializer<'de>>(parser: D) -> Result<Vec<Object>, D::Error> {
    object_intake::decode(parser)
}
fn physical_rows<'de, D: Deserializer<'de>>(parser: D) -> Result<Vec<PhysicalRow>, D::Error> {
    bounded(parser, MAX_PHYSICAL_ROWS, |_| 0, 0)
}
fn bounded<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    parser: D,
    maximum: usize,
    weight: fn(&T) -> usize,
    limit: usize,
) -> Result<Vec<T>, D::Error> {
    struct Bounded<T> {
        maximum: usize,
        weight: fn(&T) -> usize,
        limit: usize,
    }
    impl<'de, T: Deserialize<'de>> Visitor<'de> for Bounded<T> {
        type Value = Vec<T>;
        fn expecting(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            out.write_str("bounded frozen graph inventory")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            let mut done = false;
            let mut total = 0usize;
            let values = (0..self.maximum).try_fold(
                Vec::new(),
                |mut values, _| -> Result<Vec<T>, A::Error> {
                    if done {
                        return Ok(values);
                    }
                    match sequence.next_element()? {
                        Some(value) => {
                            total = total.checked_add((self.weight)(&value)).ok_or_else(|| {
                                serde::de::Error::custom("frozen inventory overflow")
                            })?;
                            if total > self.limit {
                                return Err(serde::de::Error::custom(
                                    "frozen physical inventory ceiling",
                                ));
                            }
                            values.try_reserve(1).map_err(serde::de::Error::custom)?;
                            values.push(value);
                        }
                        None => done = true,
                    }
                    Ok(values)
                },
            )?;
            if !done && sequence.next_element::<serde::de::IgnoredAny>()?.is_some() {
                return Err(serde::de::Error::custom("frozen graph inventory ceiling"));
            }
            Ok(values)
        }
    }
    parser.deserialize_seq(Bounded {
        maximum,
        weight,
        limit,
    })
}

#[cfg(test)]
mod collection_tests;
