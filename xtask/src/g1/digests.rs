
use sha2::{Digest, Sha256};

pub(crate) fn sha256_hex(blob: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(blob);
    format!("{:x}", hasher.finalize())
}

pub(crate) fn digest_head(digest: &str) -> &str {
    digest.split_at_checked(12).map_or(digest, |(head, _)| head)
}
