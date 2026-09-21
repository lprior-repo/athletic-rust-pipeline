//! Cross-artifact verification. Source fields are independently read by the field
//! verifier; the streamed XLSX annotations must also bind the verified JSONL rows.

mod binding;
mod digest;
mod pr_summary;

use crate::{
    domain::identity::WorkbookDigest,
    result_verify::{verify_results, ResultVerificationReport},
    store::ArtifactStore,
    workbook_verify::{verify_fields, VerificationReport},
};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use binding::bind_rows;
use digest::hash_file;

const MAX_DETAIL_ROW_BYTES: u64 = 128 * 1024 * 1024;
const MAX_EXCEL_CELL_UTF16_UNITS: usize = 32_767;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleVerificationReport {
    pub fields: VerificationReport,
    pub results: ResultVerificationReport,
    pub output_sha256: String,
    pub detail_sha256: String,
}

/// Verifies retained evidence consistency and revalidates raw exclusion receipts.
pub fn verify_bundle(
    original: &Path,
    output: &Path,
    expected_sha: &WorkbookDigest,
    extra_headers: &[String],
    store: &ArtifactStore,
) -> Result<BundleVerificationReport> {
    let detail = output.with_extension("jsonl");
    let output_sha256 = hash_file(output)?;
    let detail_sha256 = hash_file(&detail)?;
    let fields = verify_fields(original, output, expected_sha, extra_headers)?;
    let results = verify_results(&detail, store)?;
    if results.total_rows != fields.matched_row_count {
        bail!("sidecar row accounting differs from preserved source rows");
    }
    bind_rows(output, &detail, expected_sha, extra_headers, store)?;
    if hash_file(output)? != output_sha256
        || hash_file(&detail)? != detail_sha256
        || hash_file(original)? != expected_sha.as_str()
    {
        bail!("bundle or original changed during verification");
    }
    Ok(BundleVerificationReport {
        fields,
        results,
        output_sha256,
        detail_sha256,
    })
}
