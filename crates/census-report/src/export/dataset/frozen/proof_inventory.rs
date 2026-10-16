use super::{io, proof_directory};
use crate::export::dataset::{selected_proofs::SelectedProofsV1, ExportDataset};
use crate::report::ReportResult;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    #[serde(rename = "lineage")]
    _lineage: serde::de::IgnoredAny,
    #[serde(rename = "data")]
    _data: serde::de::IgnoredAny,
    #[serde(default)]
    selected_proofs: Option<SelectedProofsV1>,
}

impl ExportDataset {
    pub(crate) fn verify_frozen_proof_objects(path: &Path) -> ReportResult<u64> {
        let input: Inventory = io::decode_json(path, io::open_json(path)?)?;
        let mut proofs = input.selected_proofs.ok_or_else(|| {
            super::invalid("selected publication is missing its retained proof inventory")
        })?;
        proofs.bind_directory(&proof_directory(path)?)?;
        proofs.object_bytes()
    }

    pub(crate) fn verify_staged_proof_objects(&self, path: &Path) -> ReportResult<()> {
        if let Some(proofs) = &self.selected_proofs {
            proofs.verify_objects(&proof_directory(path)?)?;
        }
        Ok(())
    }
}
