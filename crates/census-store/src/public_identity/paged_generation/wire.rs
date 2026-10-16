use census_domain::model::{RunManifest, StoreIdentity};
use serde::{Deserialize, Serialize};

use super::{AuthorityTableV2, DigestV2, TableRoleV2};

pub(super) const PAGE_SCHEMA: &str = "public-identity-authority-raw-page/v2";
pub(super) const BLOCK_SCHEMA: &str = "public-identity-authority-page-refs/v2";
pub(super) const TABLE_SCHEMA: &str = "public-identity-authority-table/v2";
pub(super) const SEAL_SCHEMA: &str = "public-identity-authority-generation-seal/v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FormatBinding {
    pub store_schema: u32,
    pub key_format: u32,
    pub lifecycle_revision: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum RunBinding {
    Unbound,
    Bound { manifest: RunManifest },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CaptureBinding {
    pub store: StoreIdentity,
    pub format: FormatBinding,
    pub run: RunBinding,
    pub evidence_generation: u64,
    pub derived_generation: u64,
    pub snapshot_sequence: u64,
    pub table_revisions: [u64; 6],
    pub authority_policy: u32,
    pub enrichment_policy: u32,
    pub projection_policy: u32,
    pub claim_ordinals: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Layout {
    ObservationLog,
    DerivedGeneration { generation: u64 },
    DerivedMap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TableBinding {
    pub lineage: DigestV2,
    pub table: AuthorityTableV2,
    pub revision: u64,
    pub layout: Layout,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Inventory {
    pub pages: u32,
    pub rows: u64,
    pub raw_bytes: u64,
    pub object_count: u32,
    pub jsonl_bytes: u64,
    pub physical_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ObjectRef {
    pub root: DigestV2,
    pub bytes: u32,
    pub records: u32,
    pub physical_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Seal {
    pub schema: String,
    pub binding: CaptureBinding,
    pub tables: [TableRef; 6],
    pub inventory: Inventory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TableRef {
    pub table: AuthorityTableV2,
    pub role: TableRoleV2,
    pub binding: DigestV2,
    pub object: ObjectRef,
    pub inventory: Inventory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TableHeader {
    pub schema: String,
    pub binding: TableBinding,
    pub terminal: String,
    pub blocks: u16,
    pub inventory: Inventory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BlockRef {
    pub block: u16,
    pub first_page: u32,
    pub first_ordinal: u64,
    pub object: ObjectRef,
    pub inventory: Inventory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BlockHeader {
    pub schema: String,
    pub binding: DigestV2,
    pub table: AuthorityTableV2,
    pub block: u16,
    pub first_page: u32,
    pub first_ordinal: u64,
    pub pages: u16,
    pub rows: u64,
    pub raw_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PageRef {
    pub page: u32,
    pub first_ordinal: u64,
    pub rows: u16,
    pub raw_bytes: u32,
    pub object: ObjectRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawPageHeader {
    pub schema: String,
    pub binding: DigestV2,
    pub table: AuthorityTableV2,
    pub page: u32,
    pub first_ordinal: u64,
    pub rows: u16,
    pub raw_bytes: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawRow {
    pub physical_key_base64: String,
    pub value_bytes: u32,
    pub value_sha256: DigestV2,
    pub parts: u8,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawPart {
    pub bytes_base64: String,
}
