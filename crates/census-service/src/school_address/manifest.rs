use super::export::sha256_hex;
use super::generation_error::{io, GenerationError};
use super::report::{LaneReport, Report};
use census_domain::model::serialized_digest;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Inputs {
    pub(super) lanes: Vec<LaneReport>,
    pub(super) baseline: Option<String>,
    pub(super) ledger: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Run {
    run_id: String,
    pub(super) created_at: Option<String>,
    pub(super) inputs: Inputs,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Artifact {
    name: String,
    sha256: String,
    bytes: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    schema_revision: u32,
    run: Run,
    artifacts: Vec<Artifact>,
    pub(super) generation_digest: String,
}

#[derive(Serialize)]
struct Body<'a> {
    schema_revision: u32,
    run: &'a Run,
    artifacts: &'a [Artifact],
}

impl Manifest {
    pub(super) fn new(
        inputs: Inputs,
        month: Option<String>,
        files: &BTreeMap<String, Vec<u8>>,
    ) -> Result<Self, GenerationError> {
        let run_id = serialized_digest(&(&month, &inputs))?;
        let artifacts = files
            .iter()
            .map(|(name, bytes)| Artifact {
                name: name.clone(),
                sha256: sha256_hex(bytes),
                bytes: bytes.len(),
            })
            .collect();
        let mut manifest = Self {
            schema_revision: 1,
            run: Run {
                run_id,
                created_at: month,
                inputs,
            },
            artifacts,
            generation_digest: String::new(),
        };
        manifest.generation_digest = manifest.digest()?;
        Ok(manifest)
    }

    fn digest(&self) -> Result<String, GenerationError> {
        Ok(serialized_digest(&Body {
            schema_revision: self.schema_revision,
            run: &self.run,
            artifacts: &self.artifacts,
        })?)
    }

    fn validate(&self) -> Result<(), GenerationError> {
        if self.schema_revision != 1 {
            return Err(GenerationError::Schema(self.schema_revision));
        }
        if self.digest()? != self.generation_digest
            || self.run.run_id != serialized_digest(&(&self.run.created_at, &self.run.inputs))?
        {
            return Err(GenerationError::Digest);
        }
        let mut names = vec![
            "baseline.json",
            "school_directory.csv",
            "school_directory.json",
            "update_ledger.json",
        ];
        if self.run.inputs.baseline.is_some() {
            names.insert(1, "changes.json");
        }
        if self
            .artifacts
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>()
            != names
        {
            return Err(GenerationError::ArtifactSet);
        }
        Ok(())
    }
}

pub struct VerifiedGeneration {
    directory: PathBuf,
    files: BTreeMap<String, Vec<u8>>,
}

impl VerifiedGeneration {
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn artifact(&self, name: &str) -> Result<&[u8], GenerationError> {
        self.files
            .get(name)
            .map(Vec::as_slice)
            .ok_or_else(|| GenerationError::Artifact(name.to_string()))
    }
}

pub fn verify_current(out: &Path) -> Result<VerifiedGeneration, GenerationError> {
    let pointer = out.join("current");
    let target = std::fs::read_link(&pointer).map_err(|source| io(&pointer, source))?;
    let directory = out.join(target);
    verify_directory(out, &directory)
}

pub(super) fn verify_directory(
    out: &Path,
    directory: &Path,
) -> Result<VerifiedGeneration, GenerationError> {
    let generations =
        std::fs::canonicalize(out.join("generations")).map_err(|source| io(out, source))?;
    let root = std::fs::canonicalize(out).map_err(|source| io(out, source))?;
    if generations.parent() != Some(root.as_path())
        || generations.file_name().and_then(|name| name.to_str()) != Some("generations")
    {
        return Err(GenerationError::Pointer(generations));
    }
    let directory = std::fs::canonicalize(directory).map_err(|source| io(directory, source))?;
    if directory.parent() != Some(generations.as_path()) {
        return Err(GenerationError::Pointer(directory));
    }
    let generation = verify_staged(&directory)?;
    let manifest: Manifest = decode(
        &directory.join("manifest.json"),
        generation.artifact("manifest.json")?,
    )?;
    let expected = manifest
        .generation_digest
        .get(..16)
        .ok_or(GenerationError::Digest)?;
    if directory.file_name().and_then(|name| name.to_str()) != Some(expected) {
        return Err(GenerationError::Digest);
    }
    Ok(generation)
}

pub(super) fn verify_staged(directory: &Path) -> Result<VerifiedGeneration, GenerationError> {
    let path = directory.join("manifest.json");
    let bytes = load(&path)?;
    let manifest: Manifest = decode(&path, &bytes)?;
    manifest.validate()?;
    let mut files = BTreeMap::new();
    for artifact in &manifest.artifacts {
        let path = directory.join(&artifact.name);
        let bytes = load(&path)?;
        if bytes.len() != artifact.bytes || sha256_hex(&bytes) != artifact.sha256 {
            return Err(GenerationError::Artifact(artifact.name.clone()));
        }
        files.insert(artifact.name.clone(), bytes);
    }
    verify_report(directory, &manifest, &mut files)?;
    files.insert("manifest.json".to_string(), bytes);
    let listed = std::fs::read_dir(directory).map_err(|source| io(directory, source))?;
    for entry in listed {
        let entry = entry.map_err(|source| io(directory, source))?;
        if !files.contains_key(&entry.file_name().to_string_lossy().into_owned()) {
            return Err(GenerationError::ArtifactSet);
        }
    }
    Ok(VerifiedGeneration {
        directory: directory.to_path_buf(),
        files,
    })
}

fn verify_report(
    directory: &Path,
    manifest: &Manifest,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), GenerationError> {
    let path = directory.join("pipeline_report.json");
    let bytes = load(&path)?;
    let report: Report = decode(&path, &bytes)?;
    if report.manifest_digest != manifest.generation_digest
        || report.now != manifest.run.created_at
        || serialized_digest(&report.lanes)? != serialized_digest(&manifest.run.inputs.lanes)?
    {
        return Err(GenerationError::Report);
    }
    if report.changes.is_some() != manifest.run.inputs.baseline.is_some() {
        return Err(GenerationError::Report);
    }
    files.insert("pipeline_report.json".to_string(), bytes);
    Ok(())
}

pub(super) fn load(path: &Path) -> Result<Vec<u8>, GenerationError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| io(path, source))?;
    if !metadata.file_type().is_file() {
        return Err(GenerationError::Artifact(path.display().to_string()));
    }
    std::fs::read(path).map_err(|source| io(path, source))
}

pub(super) fn decode<T: serde::de::DeserializeOwned>(
    path: &Path,
    bytes: &[u8],
) -> Result<T, GenerationError> {
    serde_json::from_slice(bytes).map_err(|source| GenerationError::Json {
        path: path.to_path_buf(),
        source,
    })
}

pub(super) fn verified_artifact(path: &Path) -> Result<Vec<u8>, GenerationError> {
    let directory = std::fs::canonicalize(
        path.parent()
            .ok_or_else(|| GenerationError::Pointer(path.to_path_buf()))?,
    )
    .map_err(|source| io(path, source))?;
    if directory
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        != Some("generations")
    {
        return Err(GenerationError::Destination {
            path: path.to_path_buf(),
            detail: "unmanifested or legacy artifact input is rejected".to_string(),
        });
    }
    let out = directory
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| GenerationError::Pointer(directory.clone()))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| GenerationError::Pointer(path.to_path_buf()))?;
    Ok(verify_directory(out, &directory)?.artifact(name)?.to_vec())
}
