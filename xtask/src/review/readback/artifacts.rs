use super::hash::compute_file_hash;

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn verify_artifacts(bundle_dir: &Path, manifest: &Value) -> Result<Vec<Value>> {
    let artifacts = manifest["artifacts"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("manifest.artifacts is not an object"))?;

    let mut names: Vec<&str> = artifacts.keys().map(|k| k.as_str()).collect();
    names.sort();

    let mut results = Vec::new();
    for name in names {
        let Some(info) = artifacts.get(name) else {
            continue;
        };
        let declared_bytes = info["bytes"].as_u64().map_or(0, core::convert::identity);
        let declared_sha = info["sha256"].as_str().map_or("", core::convert::identity);

        let file_path = bundle_dir.join(name);
        if !file_path.exists() {
            results.push(json!({
                "name": name,
                "declared_bytes": declared_bytes,
                "actual_bytes": 0,
                "declared_sha256": declared_sha,
                "actual_sha256": "",
                "match": false,
                "missing": true
            }));
            continue;
        }

        let (actual_bytes, actual_sha) =
            compute_file_hash(&file_path).with_context(|| format!("hashing {}", name))?;

        let match_result = actual_bytes == declared_bytes && actual_sha == declared_sha;

        results.push(json!({
            "name": name,
            "declared_bytes": declared_bytes,
            "actual_bytes": actual_bytes,
            "declared_sha256": declared_sha,
            "actual_sha256": actual_sha,
            "match": match_result,
            "missing": false
        }));
    }

    Ok(results)
}
