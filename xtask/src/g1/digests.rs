//! Body digests: the one hash this report computes, and the short form its display lines print.

use sha2::{Digest, Sha256};

/// The SHA-256 of one retained body, lower-case hex.
pub(crate) fn sha256_hex(blob: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(blob);
    format!("{:x}", hasher.finalize())
}

/// The leading 12 bytes of a digest, for the display lines that shorten one.
///
/// Digests are `sha256_hex` output: 64 ASCII bytes, so the cut is always on a boundary. A shorter or
/// non-boundary string is returned whole instead of panicking.
pub(crate) fn digest_head(digest: &str) -> &str {
    digest.split_at_checked(12).map_or(digest, |(head, _)| head)
}
