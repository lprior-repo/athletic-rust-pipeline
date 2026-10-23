mod capture;

use capture::{hex, io, load, preserve, publish, refused, MAX_CAPTURE_BYTES};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use census_store::Store;
use serde::Deserialize;

use super::{JoinError, JoinReport, Overrides, Report, SchoolAddressArgs};

const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_CAPTURES: usize = 256;

type AcquisitionProvenance = BTreeMap<(String, String), (String, Option<String>)>;

#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    #[error("address replay IO at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("address replay failed: {primary}; cleanup at {path} also failed: {source}")]
    Cleanup {
        path: PathBuf,
        primary: Box<ReplayError>,
        source: std::io::Error,
    },
    #[error("address replay manifest is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("address replay refuses {path}: {detail}")]
    Refused { path: PathBuf, detail: String },
    #[error("address replay provenance: {0}")]
    Evidence(#[from] JoinError),
    #[error("address replay corpus: {0}")]
    Corpus(#[from] anyhow::Error),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_revision: u32,
    month: String,
    captures: Vec<Capture>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Capture {
    format: Format,
    path: PathBuf,
    url: String,
    retrieved_at: Option<String>,
    sha256: String,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Format {
    NcesCcd,
    NcesPss,
    StateEdIndex,
    StateEdProfile,
    StateEdTabular,
}

impl Format {
    fn provider(self) -> &'static str {
        match self {
            Self::NcesCcd => "nces-ccd",
            Self::NcesPss => "nces-pss",
            Self::StateEdIndex | Self::StateEdProfile | Self::StateEdTabular => "state-ed",
        }
    }

    fn add(self, args: &mut SchoolAddressArgs, path: PathBuf) -> Result<(), ReplayError> {
        match self {
            Self::NcesCcd if args.ccd.is_none() || args.ccd.as_deref() == Some(path.as_path()) => {
                args.ccd = Some(path)
            }
            Self::NcesPss if args.pss.is_none() || args.pss.as_deref() == Some(path.as_path()) => {
                args.pss = Some(path)
            }
            Self::StateEdIndex => args.state_ed_index.push(path),
            Self::StateEdProfile => args.state_ed_profile.push(path),
            Self::StateEdTabular => args.state_ed_tabular.push(path),
            _ => return Err(refused(&path, "duplicate NCES lane")),
        }
        Ok(())
    }
}

pub struct Replay {
    generation: PathBuf,
    digest: String,
    archive: PathBuf,
    overrides: Overrides,
}

impl Replay {
    pub fn generation_digest(&self) -> &str {
        &self.digest
    }

    pub fn archive(&self) -> &Path {
        &self.archive
    }

    pub fn overrides(&self) -> &Overrides {
        &self.overrides
    }

    pub fn apply(&self, store: &Store) -> Result<JoinReport, ReplayError> {
        capture::verify_retained(&self.archive)?;
        Ok(super::join_generation_pinned(
            store,
            &self.generation,
            self.overrides.clone(),
            &self.digest,
        )?)
    }
}

fn ensure_contained(root: &Path, path: &Path, detail: &str) -> Result<(), ReplayError> {
    let canonical_root = std::fs::canonicalize(root).map_err(|source| io(root, source))?;
    let canonical_path = std::fs::canonicalize(path).map_err(|source| io(path, source))?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(refused(path, detail));
    }
    Ok(())
}

pub fn run(manifest_path: &Path, out: &Path) -> Result<Replay, ReplayError> {
    run_scoped(manifest_path, out, super::CorpusScope::AllCaptured)
}

pub(super) fn run_scoped(
    manifest_path: &Path,
    out: &Path,
    scope: super::CorpusScope<'_>,
) -> Result<Replay, ReplayError> {
    let bytes = load(manifest_path, MAX_MANIFEST_BYTES)?;
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    if manifest.schema_revision != 1
        || manifest.captures.is_empty()
        || manifest.captures.len() > MAX_CAPTURES
    {
        return Err(refused(
            manifest_path,
            "expected schema 1 and 1..=256 captures",
        ));
    }
    let base = manifest_path
        .parent()
        .ok_or_else(|| refused(manifest_path, "no manifest parent"))?;
    let archive = out.join("captures").join(hex(&bytes));
    std::fs::create_dir_all(&archive).map_err(|source| io(&archive, source))?;
    ensure_contained(
        out,
        &archive,
        "retained capture directory escapes the output root",
    )?;
    preserve(&archive.join("manifest.json"), &bytes)?;
    let mut args = arguments(out, manifest.month);
    let acquisitions = stage_captures(base, &manifest.captures, &archive, &mut args)?;
    let overrides = Overrides::from_acquisitions(acquisitions)?;
    preserve(&archive.join(".complete"), b"")?;
    super::run_scoped(&args, scope)?;
    let generation = super::verify_current(out).map_err(JoinError::from)?;
    let report: Report = serde_json::from_slice(
        generation
            .artifact("pipeline_report.json")
            .map_err(JoinError::from)?,
    )?;
    let evidence = serde_json::to_vec_pretty(&overrides)?;
    publish(&out.join("replay-evidence.json"), &evidence)?;
    Ok(Replay {
        generation: out.to_path_buf(),
        digest: report.manifest_digest,
        archive,
        overrides,
    })
}

fn stage_captures(
    base: &Path,
    captures: &[Capture],
    archive: &Path,
    args: &mut SchoolAddressArgs,
) -> Result<Vec<super::Acquisition>, ReplayError> {
    let mut acquisitions = Vec::new();
    acquisitions
        .try_reserve_exact(captures.len())
        .map_err(|error| refused(archive, &error.to_string()))?;
    let mut seen = AcquisitionProvenance::new();
    for (i, capture) in captures.iter().enumerate() {
        if capture.path.is_absolute() {
            return Err(refused(
                &capture.path,
                "capture path must be relative to the source root",
            ));
        }
        let path = base.join(&capture.path);
        let canonical = std::fs::canonicalize(&path).map_err(|source| io(&path, source))?;
        ensure_contained(base, &canonical, "capture escapes the declared source root")?;
        let entry =
            u32::try_from(i).map_err(|_| refused(&path, "too many captures for u32 index"))?;
        acquisitions.push(stage_capture(
            capture, &canonical, archive, entry, args, &mut seen,
        )?);
    }
    Ok(acquisitions)
}

fn stage_capture(
    capture: &Capture,
    path: &Path,
    archive: &Path,
    entry: u32,
    args: &mut SchoolAddressArgs,
    seen: &mut AcquisitionProvenance,
) -> Result<super::Acquisition, ReplayError> {
    let bytes = load(path, MAX_CAPTURE_BYTES)?;
    let digest = hex(&bytes);
    if digest != capture.sha256 {
        return Err(refused(
            path,
            &format!(
                "sha256 mismatch: expected {}, observed {digest}",
                capture.sha256
            ),
        ));
    }
    retain_acquisition_provenance(seen, path, capture, &digest)?;
    let staged = archive.join(digest);
    preserve(&staged, &bytes)?;
    capture.format.add(args, staged.clone())?;
    Ok(super::Acquisition {
        entry,
        provider: capture.format.provider().to_string(),
        path: staged.display().to_string(),
        url: capture.url.clone(),
        retrieved_at: capture.retrieved_at.clone(),
    })
}

fn retain_acquisition_provenance(
    seen: &mut AcquisitionProvenance,
    path: &Path,
    capture: &Capture,
    digest: &str,
) -> Result<(), ReplayError> {
    let key = (capture.format.provider().to_string(), digest.to_string());
    let tuple = (capture.url.clone(), capture.retrieved_at.clone());
    match seen.get(&key) {
        Some(existing) if *existing != tuple => {
            let differs = match (existing.0 != tuple.0, existing.1 != tuple.1) {
                (true, true) => "url and date",
                (true, false) => "url",
                (false, true) => "date",
                (false, false) => "provenance",
            };
            Err(refused(
                path,
                &format!(
                    "capture acquisition conflict: different provenance tuples for the same body ({differs} differ)"
                ),
            ))
        }
        Some(_) => Ok(()),
        None => {
            seen.insert(key, tuple);
            Ok(())
        }
    }
}

fn arguments(out: &Path, month: String) -> SchoolAddressArgs {
    SchoolAddressArgs {
        ccd: None,
        pss: None,
        state_ed_index: Vec::new(),
        state_ed_profile: Vec::new(),
        state_ed_tabular: Vec::new(),
        associations: Vec::new(),
        association_directory: Vec::new(),
        out: out.to_path_buf(),
        baseline: None,
        ledger: None,
        now: Some(month),
        geocode: false,
        validate_postal: false,
    }
}
