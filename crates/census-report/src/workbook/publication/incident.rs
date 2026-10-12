use super::{io_error, PublicationError, ReportResult};
use census_store::publish::Digest;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Incident {
    bundle: Digest,
    diagnostics: Vec<String>,
}

pub(super) fn record(root: &Path, bundle: &Digest, diagnostics: &[String]) -> ReportResult<()> {
    let path = root.join("integrity-incident.json");
    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return check_root(root),
        Err(source) => return Err(io_error(&path, source)),
    };
    let bytes = census_domain::model::serialized_bytes(&Incident {
        bundle: bundle.clone(),
        diagnostics: diagnostics.into(),
    })
    .map_err(|error| super::invariant(format!("encoding integrity incident: {error}")))?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|source| io_error(&path, source))?;
    super::sync_directory(root)
}
pub(super) fn check_root(root: &Path) -> ReportResult<()> {
    let path = root.join("integrity-incident.json");
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(io_error(&path, source)),
        Ok(_) => {
            let incident: Incident = super::sidecars::read_json(&path)?;
            Err(PublicationError::IntegrityIncident {
                bundle: incident.bundle,
                detail: incident.diagnostics.join("; "),
            }
            .into())
        }
    }
}
pub(super) fn check_workbook(path: &Path) -> ReportResult<()> {
    let path = std::fs::canonicalize(path).map_err(|source| io_error(path, source))?;
    let root = path
        .parent()
        .and_then(Path::parent)
        .filter(|parent| parent.file_name().is_some_and(|name| name == "generations"))
        .and_then(Path::parent)
        .ok_or_else(|| super::invariant("seal requires an owned publication generation".into()))?;
    check_root(root)
}
