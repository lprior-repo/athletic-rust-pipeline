use super::errors::io_err;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const DIGEST_BYTES: usize = 64;

pub(crate) fn http_cache_integrity(root: &Path) -> crate::StoreResult<Vec<String>> {
    let http = root.join("http");
    if !http.exists() {
        return Ok(Vec::new());
    }
    let mut issues = Vec::new();
    check_live_entries(&http, &mut issues)?;
    check_archive(&http.join("archive"), &mut issues)?;
    Ok(issues)
}

fn check_live_entries(http: &Path, issues: &mut Vec<String>) -> crate::StoreResult<()> {
    for entry in fs::read_dir(http).map_err(|source| io_err(http, source))? {
        let entry = entry.map_err(|source| io_err(http, source))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry.file_type().map_err(|source| io_err(http, source))?;
        if kind.is_dir() {
            if !matches!(name.as_str(), "archive" | "quarantine")
                && !name.starts_with(".capture-stage")
            {
                issues.push(format!("http: unexpected directory {name}"));
            }
            continue;
        }
        if !kind.is_file() {
            issues.push(format!("http: unexpected entry {name}"));
            continue;
        }
        if name.ends_with(".json.lock") {
            continue;
        }
        if let Some(key) = name.strip_suffix(".body") {
            if !http.join(format!("{key}.meta.json")).exists() {
                issues.push(format!("http: body {name} has no metadata"));
            }
        } else if let Some(key) = name.strip_suffix(".meta.json") {
            check_live_pair(http, key, &name, issues)?;
        } else {
            issues.push(format!("http: unexpected file {name}"));
        }
    }
    Ok(())
}

fn check_live_pair(
    http: &Path,
    key: &str,
    name: &str,
    issues: &mut Vec<String>,
) -> crate::StoreResult<()> {
    let body = http.join(format!("{key}.body"));
    if !body.exists() {
        issues.push(format!("http: metadata {name} has no body"));
        return Ok(());
    }
    let Some(digest) = read_meta_digest(&http.join(format!("{key}.meta.json")), name, issues)?
    else {
        return Ok(());
    };
    verify_body(&body, &digest, name, issues)
}

fn check_archive(archive: &Path, issues: &mut Vec<String>) -> crate::StoreResult<()> {
    if !archive.exists() {
        return Ok(());
    }
    let mut has_bodies = false;
    let mut has_captures = false;
    for entry in fs::read_dir(archive).map_err(|source| io_err(archive, source))? {
        let entry = entry.map_err(|source| io_err(archive, source))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry
            .file_type()
            .map_err(|source| io_err(archive, source))?;
        match name.as_str() {
            "bodies" if kind.is_dir() => has_bodies = true,
            "captures" if kind.is_dir() => has_captures = true,
            _ => issues.push(format!("archive: unexpected entry {name}")),
        }
    }
    if !has_bodies {
        issues.push("archive: bodies directory is missing".to_string());
    }
    if !has_captures {
        issues.push("archive: captures directory is missing".to_string());
    }
    let bodies = archive.join("bodies");
    let captures = archive.join("captures");
    let body_digests = check_archive_bodies(&bodies, issues)?;
    let referenced = check_archive_captures(&captures, &bodies, issues)?;
    for digest in &body_digests {
        if !referenced.contains(digest) {
            issues.push(format!(
                "archive/bodies: {digest}.body has no capture metadata"
            ));
        }
    }
    Ok(())
}

fn check_archive_bodies(
    bodies: &Path,
    issues: &mut Vec<String>,
) -> crate::StoreResult<BTreeSet<String>> {
    let mut digests = BTreeSet::new();
    if !bodies.exists() {
        return Ok(digests);
    }
    for entry in fs::read_dir(bodies).map_err(|source| io_err(bodies, source))? {
        let entry = entry.map_err(|source| io_err(bodies, source))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry.file_type().map_err(|source| io_err(bodies, source))?;
        if !kind.is_file() {
            issues.push(format!("archive/bodies: unexpected entry {name}"));
            continue;
        }
        let Some(digest) = name.strip_suffix(".body") else {
            issues.push(format!("archive/bodies: unexpected file {name}"));
            continue;
        };
        if !is_digest(digest) {
            issues.push(format!("archive/bodies: invalid digest name {name}"));
            continue;
        }
        verify_body(
            &entry.path(),
            digest,
            &format!("archive/bodies/{name}"),
            issues,
        )?;
        digests.insert(digest.to_string());
    }
    Ok(digests)
}

fn check_archive_captures(
    captures: &Path,
    bodies: &Path,
    issues: &mut Vec<String>,
) -> crate::StoreResult<BTreeSet<String>> {
    let mut referenced = BTreeSet::new();
    if !captures.exists() {
        return Ok(referenced);
    }
    for entry in fs::read_dir(captures).map_err(|source| io_err(captures, source))? {
        let entry = entry.map_err(|source| io_err(captures, source))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry
            .file_type()
            .map_err(|source| io_err(captures, source))?;
        if !kind.is_dir() {
            issues.push(format!("archive/captures: unexpected entry {name}"));
            continue;
        }
        if !is_digest(&name) {
            issues.push(format!("archive/captures: invalid digest directory {name}"));
            continue;
        }
        let matched = check_capture_directory(&entry.path(), &name, issues)?;
        if !matched {
            issues.push(format!(
                "archive/captures/{name}: capture metadata is missing"
            ));
        }
        if bodies.join(format!("{name}.body")).exists() {
            referenced.insert(name);
        } else {
            issues.push(format!(
                "archive/captures/{name}: referenced body is missing"
            ));
        }
    }
    Ok(referenced)
}

fn check_capture_directory(
    directory: &Path,
    digest: &str,
    issues: &mut Vec<String>,
) -> crate::StoreResult<bool> {
    let mut matched = false;
    for entry in fs::read_dir(directory).map_err(|source| io_err(directory, source))? {
        let entry = entry.map_err(|source| io_err(directory, source))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry
            .file_type()
            .map_err(|source| io_err(directory, source))?;
        if !kind.is_file() {
            issues.push(format!(
                "archive/captures/{digest}: unexpected entry {name}"
            ));
            continue;
        }
        let Some(metadata_digest) = name.strip_suffix(".meta.json") else {
            issues.push(format!("archive/captures/{digest}: unexpected file {name}"));
            continue;
        };
        if !is_digest(metadata_digest) {
            issues.push(format!(
                "archive/captures/{digest}: invalid metadata name {name}"
            ));
            continue;
        }
        let path = entry.path();
        let Ok(bytes) = fs::read(&path) else {
            issues.push(format!("archive/captures/{digest}: unreadable {name}"));
            continue;
        };
        if hex_digest(&bytes) != metadata_digest {
            issues.push(format!(
                "archive/captures/{digest}: metadata bytes do not match {name}"
            ));
        }
        match parse_meta_digest(&bytes, &format!("archive/captures/{digest}/{name}"), issues)? {
            Some(content) if content == digest => matched = true,
            Some(_) => issues.push(format!(
                "archive/captures/{digest}: metadata references another digest in {name}"
            )),
            None => {}
        }
    }
    Ok(matched)
}

fn read_meta_digest(
    path: &Path,
    label: &str,
    issues: &mut Vec<String>,
) -> crate::StoreResult<Option<String>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => {
            issues.push(format!("meta {label}: unreadable"));
            return Ok(None);
        }
    };
    parse_meta_digest(&bytes, label, issues)
}

fn parse_meta_digest(
    bytes: &[u8],
    label: &str,
    issues: &mut Vec<String>,
) -> crate::StoreResult<Option<String>> {
    let Ok(meta) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        issues.push(format!("meta {label}: invalid JSON"));
        return Ok(None);
    };
    match meta.get("content_digest").and_then(|value| value.as_str()) {
        Some(value) if is_digest(value) => Ok(Some(value.to_string())),
        _ => {
            issues.push(format!("meta {label}: missing or invalid content_digest"));
            Ok(None)
        }
    }
}

fn verify_body(
    path: &Path,
    expected: &str,
    label: &str,
    issues: &mut Vec<String>,
) -> crate::StoreResult<()> {
    let bytes = fs::read(path).map_err(|source| io_err(path, source))?;
    if hex_digest(&bytes) != expected {
        issues.push(format!("body {label}: digest mismatch"));
    }
    Ok(())
}

fn hex_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_digest(value: &str) -> bool {
    value.len() == DIGEST_BYTES
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
#[path = "http_cache_tests.rs"]
mod tests;
