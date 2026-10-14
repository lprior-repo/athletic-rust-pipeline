use super::Digest;
use crate::{StoreError, StoreResult};
#[cfg(any(target_os = "linux", target_os = "macos", all(test, unix)))]
use sha2::{Digest as _, Sha256};

#[cfg(any(target_os = "macos", all(test, unix)))]
#[path = "lease_boot/macos.rs"]
mod macos;

#[cfg(target_os = "linux")]
pub(super) fn read_owner_boot_digest() -> StoreResult<Digest> {
    let path = std::path::PathBuf::from("/proc/sys/kernel/random/boot_id");
    let bytes = std::fs::read(&path).map_err(|source| StoreError::Io { path, source })?;
    hash_boot_bytes(&bytes)
}

#[cfg(target_os = "macos")]
pub(super) fn read_owner_boot_digest() -> StoreResult<Digest> {
    macos::read_owner_boot_digest()
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(super) fn read_owner_boot_digest() -> StoreResult<Digest> {
    Err(StoreError::Refused {
        detail: "lease boot-session identity is unsupported on this operating system".into(),
    })
}

#[cfg(any(target_os = "linux", target_os = "macos", all(test, unix)))]
fn hash_boot_bytes(bytes: &[u8]) -> StoreResult<Digest> {
    Digest::parse(&format!("{:x}", Sha256::digest(bytes)))
}
