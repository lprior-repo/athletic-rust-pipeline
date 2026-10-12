use super::{io_error, manifest, sync_directory, PublicationError, ReportResult};
use census_store::publish::{Digest, Root};
use std::path::{Component, Path};

pub(super) fn read(root: &Path) -> ReportResult<Root> {
    let path = root.join("current");
    let relative = match std::fs::read_link(&path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Root::Absent),
        Err(error) => return Err(required(root, error.to_string(), Vec::new())),
    };
    let mut parts = relative.components();
    if parts.next() != Some(Component::Normal("generations".as_ref())) {
        return Err(required(
            root,
            "pointer is not an owned relative link".into(),
            Vec::new(),
        ));
    }
    let digest = match parts.next() {
        Some(Component::Normal(name)) => name.to_str().and_then(|name| Digest::parse(name).ok()),
        _ => None,
    };
    match (digest, parts.next()) {
        (Some(digest), None) => Ok(Root::Bundle(digest)),
        _ => Err(required(
            root,
            "pointer has an invalid generation digest".into(),
            Vec::new(),
        )),
    }
}

pub(super) fn validate(root: &Path, digest: &Digest) -> ReportResult<std::fs::File> {
    let directory = root.join("generations").join(digest.as_str());
    let metadata =
        std::fs::symlink_metadata(&directory).map_err(|source| io_error(&directory, source))?;
    if !metadata.file_type().is_dir() {
        return Err(required(
            root,
            "retained generation is not a real directory".into(),
            vec![Root::Bundle(digest.clone())],
        ));
    }
    let pin = std::fs::File::open(&directory).map_err(|source| io_error(&directory, source))?;
    pin.lock_shared()
        .map_err(|source| io_error(&directory, source))?;
    let verified = manifest::verify_directory(&directory)?;
    if verified.generation_digest != digest.as_str() {
        return Err(required(
            root,
            "bundle digest differs from selected directory".into(),
            vec![Root::Bundle(digest.clone())],
        ));
    }
    Ok(pin)
}
pub(super) fn validate_binding(
    root: &Path,
    digest: &Digest,
    input: &census_store::publish::InputFence,
) -> ReportResult<()> {
    let path = root
        .join("generations")
        .join(digest.as_str())
        .join("manifest.json");
    let manifest: manifest::Manifest = super::sidecars::read_json(&path)?;
    if manifest.lineage.store_identity != input.lineage.as_str()
        || super::binding::input_token(&manifest.lineage)? != input.token
        || super::binding::recipe(&manifest.lineage, &manifest.selection()?)? != input.recipe
    {
        return Err(PublicationError::InputFenceRefused {
            detail: "verified bundle differs from persisted lineage, token or recipe".into(),
        }
        .into());
    }
    Ok(())
}

pub(super) fn switch(root: &Path, digest: &Digest) -> ReportResult<()> {
    let nonce = super::staging_nonce(digest.as_str())?;
    let temporary = root.join(format!(".current.{nonce}.tmp"));
    std::os::unix::fs::symlink(Path::new("generations").join(digest.as_str()), &temporary)
        .map_err(|source| io_error(&temporary, source))?;
    match std::fs::rename(&temporary, root.join("current")) {
        Ok(()) => Ok(()),
        Err(source) => {
            let error = io_error(&root.join("current"), source);
            match std::fs::remove_file(&temporary) {
                Ok(()) => Err(error),
                Err(source) => Err(crate::report::ReportError::Cleanup {
                    operation: Box::new(error),
                    cleanup: Box::new(io_error(&temporary, source)),
                }),
            }
        }
    }
}

pub(super) fn finish(
    root: &Path,
    intent: &census_store::publish::Intent,
    mark: impl FnOnce() -> ReportResult<()>,
) -> ReportResult<()> {
    super::checkpoint::reached("renamed", root)
        .and_then(|()| sync_directory(root))
        .and_then(|()| mark())
        .map_err(|source| super::protocol::uncertain(intent, source))
}

pub(super) fn required(
    root: &Path,
    detail: String,
    retained: Vec<Root>,
) -> crate::report::ReportError {
    PublicationError::RecoveryRequired {
        root: root.into(),
        detail,
        retained,
    }
    .into()
}
