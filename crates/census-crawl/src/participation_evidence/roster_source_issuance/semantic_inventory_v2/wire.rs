use crate::milesplit::{RosterRejectionKind, SourceRowLocator};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Inventory {
    pub(super) schema: String,
    pub(super) policy: String,
    pub(super) provider_semantic_contract: String,
    pub(super) kernel_document: Cow<'static, str>,
    pub(super) packet: String,
    pub(super) parser_manifest: String,
    pub(super) metadata: String,
    pub(super) context: String,
    pub(super) disposition: Disposition,
    pub(super) rows: Vec<Row>,
}
#[derive(PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Disposition {
    Complete,
    Partial,
}
#[derive(PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Row {
    pub(super) row: SourceRowLocator,
    pub(super) provider_id: String,
    pub(super) outcome: Outcome,
}
#[derive(PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Outcome {
    Admitted { semantic_digest: String },
    Rejected { reason: RosterRejectionKind },
}
