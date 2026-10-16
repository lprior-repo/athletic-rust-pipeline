use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::{digest, key, Rows, TestResult};

pub(super) fn verify(journal: &Rows, descriptor: &Value, complete: bool) -> TestResult<Rows> {
    let physical = &descriptor["physical"];
    let object = &descriptor["object"];
    let root = physical["root"].as_str().ok_or("physical root")?;
    check!(eq; physical["root"], object["root"]);
    check!(eq; physical["full_bytes"], object["physical_bytes"]);
    check!(eq; physical["jsonl_bytes"], object["bytes"]);
    check!(eq; physical["records"], object["records"]);
    let mut rows = Rows::new();
    let mut full_bytes = 0u64;
    let mut bytes = Vec::new();
    let chunks = physical["chunks"].as_array().ok_or("chunk fingerprints")?;
    let pages = physical["pages"].as_array().ok_or("index fingerprints")?;
    check!(chunks.len() <= 4096 && pages.len() <= 4096);
    for (segment, fingerprints) in [("chunk", chunks.as_slice()), ("index", pages.as_slice())] {
        for (index, fingerprint) in fingerprints.iter().enumerate() {
            let key = key(segment, root, Some(index))?;
            full_bytes = full_bytes
                .checked_add(u64::try_from(key.len())?)
                .and_then(|sum| sum.checked_add(fingerprint["length"].as_u64()?))
                .ok_or("physical bytes")?;
            if let Some(raw) = journal.get(&key) {
                fingerprint_matches(fingerprint, raw)?;
                if segment == "chunk" {
                    let chunk: Value = serde_json::from_slice(raw)?;
                    let decoded =
                        BASE64_STANDARD.decode(chunk["data"].as_str().ok_or("chunk data")?)?;
                    check!(eq; chunk["length"], json!(decoded.len()));
                    check!(eq; chunk["digest"], json!(digest(&decoded)));
                    check!(eq; chunk["object"], json!(root));
                    check!(eq; chunk["index"], json!(index));
                    bytes.extend_from_slice(&decoded);
                }
                rows.insert(key, raw.clone());
            } else {
                check!(!complete, "complete graph object lacks physical row");
            }
        }
    }
    let manifest_key = key("manifest", root, None)?;
    full_bytes = full_bytes
        .checked_add(u64::try_from(manifest_key.len())?)
        .and_then(|sum| sum.checked_add(physical["manifest"]["length"].as_u64()?))
        .ok_or("manifest bytes")?;
    check!(eq; physical["full_bytes"], json!(full_bytes));
    if let Some(raw) = journal.get(&manifest_key) {
        fingerprint_matches(&physical["manifest"], raw)?;
        rows.insert(manifest_key, raw.clone());
        check!(eq; bytes.len(), usize::try_from(object["bytes"].as_u64().ok_or("object bytes")?)?);
        check!(eq; digest(&bytes), root);
        check!(eq; bytes.split_inclusive(|byte| *byte == b'\n').count(), usize::try_from(object["records"].as_u64().ok_or("object records")?)?);
    } else {
        check!(!complete, "complete graph object lacks manifest");
    }
    let prefixes = [
        format!("j2\0chunk\0{root}\0"),
        format!("j2\0index\0{root}\0"),
    ];
    for (key, value) in journal.iter().filter(|(key, _)| {
        prefixes
            .iter()
            .any(|prefix| key.starts_with(prefix.as_bytes()))
    }) {
        check!(eq; rows.get(key), Some(value));
    }
    Ok(rows)
}

fn fingerprint_matches(fingerprint: &Value, raw: &[u8]) -> TestResult {
    check!(eq; fingerprint["length"], json!(raw.len()));
    check!(eq; fingerprint["digest"], json!(Sha256::digest(raw).to_vec()));
    Ok(())
}
