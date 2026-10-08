use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct ReceiptIndex {
    pub(super) by_path: HashMap<String, (String, String, String)>,
    pub(super) by_digest: HashMap<String, Vec<(String, String)>>,
}

pub(super) fn path_key(path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    hasher
        .finalize()
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) fn receipt_key(meet: &str, role: &str, path_key: &str, digest: &str) -> String {
    format!("{meet}:{role}:{path_key}:{digest}")
}

pub(super) fn parse_receipt_key(key: &str) -> Option<(String, String, String, String)> {
    let mut parts = key.splitn(4, ':');
    let meet = parts.next()?;
    let role = parts.next()?;
    let path = parts.next()?;
    let digest = parts.next()?;
    let hex = |value: &str, len: usize| {
        value.len() == len && value.bytes().all(|byte| byte.is_ascii_hexdigit())
    };
    let valid = !meet.is_empty()
        && meet.bytes().all(|byte| byte.is_ascii_digit())
        && !role.is_empty()
        && hex(path, 16)
        && hex(digest, 64);
    valid.then(|| {
        (
            meet.to_string(),
            role.to_string(),
            path.to_string(),
            digest.to_string(),
        )
    })
}
