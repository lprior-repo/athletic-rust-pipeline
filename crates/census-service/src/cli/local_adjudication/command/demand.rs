use super::{emit, open};
use anyhow::{Context, Result};
use std::path::Path;

pub(super) fn run(root: &Path) -> Result<()> {
    let (store, _writer) = open(root)?;
    let snapshot = store.snapshot();
    let measurement = snapshot
        .measure_frozen_public_identity_v2()
        .context("measuring frozen identity demand; no frozen bundle or publication was created")?;
    emit(&serde_json::json!({
        "schema": "frozen-public-identity-json-demand/v2",
        "status": "complete_authenticated_byte_walk",
        "measurement": measurement,
        "qualification_scope": "Canonical JSON length only. The finite diagnostic JSON limit excludes qualification and graph verification I/O, raw physical bytes and RSS. Completion does not certify final encode, frozen replay, identical accepted projection, workbook or publication. Numeric fit is not acceptance.",
    }))
}
