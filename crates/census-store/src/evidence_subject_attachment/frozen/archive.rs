use super::super::{RetainedCaptureRootSchemaV1, OPERATION_PREFIX, PHASE};
use super::*;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::value::RawValue;
use std::fmt;

mod array;
mod equality;
mod read;
mod state;
mod validate;
pub(super) use read::read;

#[derive(Serialize)]
pub(super) struct Archive<'a> {
    pub(super) schema: &'a str,
    pub(super) store: &'a str,
    pub(super) snapshot_sequence: u64,
    pub(super) public_head: Head<'a>,
    pub(super) inventory: Inventory<'a>,
    pub(super) prefixes: Vec<Prefix<'a>>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Head<'a> {
    pub(super) sequence: u64,
    #[serde(borrow)]
    pub(super) digest: &'a str,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Inventory<'a> {
    pub(super) version: u8,
    pub(super) records: u64,
    #[serde(borrow)]
    pub(super) digest: &'a str,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Kind {
    Attachment,
    Receipts,
    External,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reference<'a> {
    #[serde(borrow)]
    pub(super) digest: &'a str,
    pub(super) bytes: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Root<'a> {
    pub(super) schema: RetainedCaptureRootSchemaV1,
    #[serde(borrow)]
    pub(super) key: &'a str,
    #[serde(borrow)]
    pub(super) payload: Reference<'a>,
}

#[derive(Serialize)]
pub(super) struct Prefix<'a> {
    pub(super) kind: Kind,
    pub(super) root: Option<Root<'a>>,
    pub(super) physical_rows: u64,
    pub(super) encoded_bytes: u64,
    pub(super) rows: Vec<Row<'a>>,
}

#[derive(Serialize)]
pub(super) struct Row<'a> {
    #[serde(serialize_with = "serialize_key")]
    pub(super) key: Vec<u8>,
    pub(super) value: &'a RawValue,
}

pub(super) fn serialize_key<S: Serializer>(key: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(&Hex(key))
}

struct Hex<'a>(&'a [u8]);
impl fmt::Display for Hex<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[derive(Serialize)]
pub(super) struct BorrowedRow<'a> {
    #[serde(serialize_with = "serialize_key")]
    pub(super) key: &'a [u8],
    pub(super) value: &'a RawValue,
}
