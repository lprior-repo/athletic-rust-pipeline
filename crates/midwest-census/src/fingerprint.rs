//! Deterministic content fingerprints for change detection.
//!
//! Deliberately outside `census-domain`: the fingerprint is taken over the *serialized JSON* form,
//! so it carries the `serde_json` dependency that the pure domain crate must not have.

use serde::Serialize;
use sha2::{Digest, Sha256};

/// A deterministic fingerprint of an entity's identifying fields, used by change detection.
pub fn content_fingerprint<T: Serialize>(value: &T) -> String {
    let json = serde_json::to_string(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    let digest = hasher.finalize();
    // `take(12)` is the 96-bit fingerprint prefix (SHA-256 always yields 32 bytes).
    digest.iter().take(12).map(|b| format!("{b:02x}")).collect()
}
