use crate::milesplit::{RosterRejectionKind, SourceRowLocator};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Inventory {
    pub(super) schema: String,
    pub(super) policy: String,
    pub(super) provider_semantic_contract: String,
    pub(super) kernel_document: String,
    pub(super) packet: String,
    pub(super) parser_manifest: String,
    pub(super) metadata: String,
    pub(super) context: String,
    pub(super) disposition: Disposition,
    pub(super) rows: Vec<Row>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Disposition {
    Complete,
    Partial,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Row {
    pub(super) row: SourceRowLocator,
    pub(super) provider_id: String,
    pub(super) outcome: Outcome,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Outcome {
    Admitted { semantic_digest: String },
    Rejected { reason: RosterRejectionKind },
}
