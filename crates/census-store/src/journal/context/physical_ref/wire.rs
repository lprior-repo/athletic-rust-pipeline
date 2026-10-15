use serde::{
    de::{SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use std::fmt;

use super::{ContextPhysicalRef, RowFingerprint};
use crate::journal::context::{CONTEXT_CHUNKS, CONTEXT_PAGES};
use crate::{StoreError, StoreResult};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PhysicalRef {
    #[serde(deserialize_with = "root")]
    root: String,
    full_bytes: u64,
    jsonl_bytes: u64,
    records: u64,
    manifest: RowFingerprint,
    #[serde(deserialize_with = "chunks")]
    chunks: Vec<RowFingerprint>,
    #[serde(deserialize_with = "pages")]
    pages: Vec<RowFingerprint>,
}

impl TryFrom<PhysicalRef> for ContextPhysicalRef {
    type Error = StoreError;

    fn try_from(raw: PhysicalRef) -> StoreResult<Self> {
        let result = Self {
            root: raw.root,
            full_bytes: raw.full_bytes,
            jsonl_bytes: raw.jsonl_bytes,
            records: raw.records,
            manifest: raw.manifest,
            chunks: raw.chunks,
            pages: raw.pages,
        };
        result.validate()?;
        Ok(result)
    }
}

fn root<'de, D: Deserializer<'de>>(parser: D) -> Result<String, D::Error> {
    struct Root;
    impl Visitor<'_> for Root {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a lowercase SHA256 context root")
        }

        fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
            if !crate::journal::is_object_id(value) {
                return Err(E::custom("invalid physical context root"));
            }
            let mut root = String::new();
            root.try_reserve_exact(64).map_err(E::custom)?;
            root.push_str(value);
            Ok(root)
        }
    }
    parser.deserialize_str(Root)
}

fn chunks<'de, D: Deserializer<'de>>(parser: D) -> Result<Vec<RowFingerprint>, D::Error> {
    rows::<D, CONTEXT_CHUNKS>(parser)
}

fn pages<'de, D: Deserializer<'de>>(parser: D) -> Result<Vec<RowFingerprint>, D::Error> {
    rows::<D, CONTEXT_PAGES>(parser)
}

fn rows<'de, D: Deserializer<'de>, const MAX: usize>(
    parser: D,
) -> Result<Vec<RowFingerprint>, D::Error> {
    struct Rows<const LIMIT: usize>;
    impl<'de, const LIMIT: usize> Visitor<'de> for Rows<LIMIT> {
        type Value = Vec<RowFingerprint>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a bounded physical context row inventory")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Self::Value, A::Error> {
            let mut rows = Vec::new();
            for _ in 0..LIMIT {
                match sequence.next_element()? {
                    Some(row) => {
                        rows.try_reserve_exact(1)
                            .map_err(serde::de::Error::custom)?;
                        rows.push(row);
                    }
                    None => return Ok(rows),
                }
            }
            if sequence.next_element::<RowFingerprint>()?.is_some() {
                return Err(serde::de::Error::custom(
                    "physical context row count exceeds bound",
                ));
            }
            Ok(rows)
        }
    }
    parser.deserialize_seq(Rows::<MAX>)
}
