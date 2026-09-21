use super::EXPORT_PROTOCOL_REVISION;
use crate::domain::identity::EvidenceDigest;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub run: EvidenceDigest,
    pub destination: PathBuf,
}

impl ExportRequest {
    /// One destination has one SDK owner, including requests from different runs.
    pub fn key(&self) -> Result<String> {
        let destination = normalize_destination(&self.destination)?;
        super::super::identity::fingerprint(&(EXPORT_PROTOCOL_REVISION, destination))
            .map(|digest| digest.as_str().to_owned())
    }
}

pub(super) fn normalize_destination(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        bail!("export destination must be absolute");
    }
    let text = path
        .to_str()
        .context("export destination must be valid UTF-8")?;
    if text.chars().any(char::is_control) {
        bail!("export destination contains control characters");
    }
    if !path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("xlsx"))
    {
        bail!("export destination must have an XLSX extension");
    }
    if path.file_name().is_none() {
        bail!("export destination must name a file");
    }
    path.components()
        .try_fold(PathBuf::new(), |mut normalized, component| {
            match component {
                Component::RootDir => normalized.push(Path::new("/")),
                Component::Normal(value) => normalized.push(value),
                Component::CurDir => {}
                Component::ParentDir => bail!("export destination traversal is not allowed"),
                Component::Prefix(_) => bail!("unsupported export destination prefix"),
            }
            Ok(normalized)
        })
}
