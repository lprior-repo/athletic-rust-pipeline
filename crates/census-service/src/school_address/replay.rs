mod capture;

use capture::{hex, io, load, preserve, publish, refused, MAX_CAPTURE_BYTES};
use std::path::{Path, PathBuf};

use census_store::Store;
use serde::Deserialize;

use super::{JoinError, JoinReport, Overrides, Report, SchoolAddressArgs};

const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
const MAX_CAPTURES: usize = 256;

#[derive(Debug, thiserror::Error)]
pub enum ReplayError {
    #[error("address replay IO at {path}: {source}")]
    Io {
        path: PathBuf,
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
            Self::NcesCcd
                if args.ccd.is_none() || args.ccd.as_deref() == Some(path.as_path()) =>
            {
                args.ccd = Some(path)
            }
            Self::NcesPss
                if args.pss.is_none() || args.pss.as_deref() == Some(path.as_path()) =>
            {
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

fn ensure_contained(out: &Path, archive: &Path) -> Result<(), ReplayError> {
    let root = std::fs::canonicalize(out).map_err(|source| io(out, source))?;
    let directory = std::fs::canonicalize(archive).map_err(|source| io(archive, source))?;
    if !directory.starts_with(&root) {
        return Err(refused(
            archive,
            "retained capture directory escapes the output root",
        ));
    }
    Ok(())
}

pub fn run(manifest_path: &Path, out: &Path) -> Result<Replay, ReplayError> {
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
    let mut args = arguments(out, manifest.month);
    let mut overrides = Overrides::default();
    let mut seen: std::collections::BTreeMap<PathBuf, (String, Option<String>)> = std::collections::BTreeMap::new();
    let base = manifest_path
        .parent()
        .ok_or_else(|| refused(manifest_path, "no manifest parent"))?;
    let archive = out.join("captures").join(hex(&bytes));
    std::fs::create_dir_all(&archive).map_err(|source| io(&archive, source))?;
    ensure_contained(out, &archive)?;
    preserve(&archive.join("manifest.json"), &bytes)?;

    for (i, capture) in manifest.captures.iter().enumerate() {
        let path = base.join(&capture.path);
        let canonical = std::fs::canonicalize(&path).map_err(|source| io(&path, source))?;
        let tuple = (capture.url.clone(), capture.retrieved_at.clone());
        match seen.get(&canonical) {
            Some(existing) if *existing != tuple => {
                let differs = match (existing.0 != tuple.0, existing.1 != tuple.1) {
                    (true, true) => "url and date",
                    (true, false) => "url",
                    (false, true) => "date",
                    (false, false) => "provenance",
                };
                return Err(refused(
                    &path,
                    &format!(
                        "capture acquisition conflict: different provenance tuples for the same body ({differs} differ)"
                    ),
                ));
            }
            Some(_) => {} // identical tuple, duplicate entry - accept
            None => {
                seen.insert(canonical, tuple);
            }
        }
        let entry = u32::try_from(i)
            .map_err(|_| refused(&path, "too many captures for u32 index"))?;
        stage_capture(capture, &path, &archive, entry, &mut args, &mut overrides)?;
    }

    let overrides = overrides.validated()?;
    super::run(&args)?;
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

fn stage_capture(
    capture: &Capture,
    path: &Path,
    archive: &Path,
    entry: u32,
    args: &mut SchoolAddressArgs,
    overrides: &mut Overrides,
) -> Result<(), ReplayError> {
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
    let staged = archive.join(digest);
    let provider = capture.format.provider();

    overrides.acquisitions.push(super::Acquisition {
        entry,
        provider: provider.to_string(),
        path: staged.display().to_string(),
        url: capture.url.clone(),
        retrieved_at: capture.retrieved_at.clone(),
    });

    preserve(&staged, &bytes)?;
    capture.format.add(args, staged)?;
    Ok(())
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
