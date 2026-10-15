use crate::public_identity::{retained::Revision, PublicIdentityError};
use crate::{StoreError, StoreResult};
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::ReplayLimits;
use serde::{
    de::{SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::fmt;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Bundle {
    pub version: u32,
    pub store_identity: StoreIdentity,
    pub revision: Revision,
    pub declaration: String,
    pub head: String,
    pub history: String,
    #[serde(deserialize_with = "bounded")]
    pub operations: Vec<Operation>,
    #[serde(deserialize_with = "bounded")]
    pub contexts: Vec<Context>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Operation {
    pub sequence: u64,
    pub witness: String,
    pub receipt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Context {
    pub root: String,
    pub bytes: String,
}

impl Bundle {
    pub(super) fn bounds(&self) -> StoreResult<()> {
        if self.version != 1 {
            return Err(PublicIdentityError::Corrupt("frozen identity bundle version").into());
        }
        bound(self.declaration.len(), 65_536)?;
        bound(self.head.len(), 1024)?;
        bound(self.history.len(), crate::MAX_PUBLIC_IDENTITY_HISTORY_BYTES)?;
        bound(self.operations.len(), ReplayLimits::SMALL.records)?;
        bound(self.contexts.len(), ReplayLimits::SMALL.records)?;
        for operation in &self.operations {
            crate::public_identity::operation::HeldWitness::decode(
                "frozen witness bounds",
                operation.witness.as_bytes(),
            )?;
            bound(operation.receipt.len(), 65_536)?;
            if let Some(decision) = &operation.decision {
                bound(decision.len(), 65_536)?;
            }
        }
        for context in &self.contexts {
            bound(context.root.len(), 64)?;
            bound(
                context.bytes.len(),
                crate::journal::context::CONTEXT_TOTAL_BYTES,
            )?;
        }
        Ok(())
    }
}

pub(super) fn bound(bytes: usize, max: usize) -> StoreResult<()> {
    if bytes > max {
        return Err(StoreError::RowTooLarge { bytes, max });
    }
    Ok(())
}

fn bounded<'de, D: Deserializer<'de>, T: Deserialize<'de>>(parser: D) -> Result<Vec<T>, D::Error> {
    struct Bounded<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for Bounded<T> {
        type Value = Vec<T>;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a bounded identity bundle collection")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            let mut values = Vec::new();
            while let Some(value) = sequence.next_element()? {
                if values.len() >= ReplayLimits::SMALL.records {
                    return Err(serde::de::Error::custom(
                        "frozen identity collection exceeds bound",
                    ));
                }
                values.try_reserve(1).map_err(serde::de::Error::custom)?;
                values.push(value);
            }
            Ok(values)
        }
    }
    parser.deserialize_seq(Bounded(std::marker::PhantomData))
}
