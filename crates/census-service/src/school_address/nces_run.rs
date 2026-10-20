//! Run-bound, offline composition of the existing NCES address reader and retained replay.
//!
//! This module accepts operator-selected CCD/PSS files for the already-bound 2026/class-of-2027
//! five-state run. It does not acquire bytes, discover URLs, or claim source coverage.

use anyhow::{bail, Context, Result};
use census_domain::model::{
    serialized_digest, NcesAddressRunReceiptV1, NcesArtifactProviderV1, NcesArtifactReferenceV1,
    RunManifest,
};
use census_domain::school_directory::SchoolDirectoryEntry;
use census_domain::UsJurisdiction;
use census_store::Store;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

const MAX_ARTIFACT_BYTES: u64 = 128 * 1024 * 1024;
const FIVE_STATE_SCOPE: [UsJurisdiction; 5] = [
    UsJurisdiction::Michigan,
    UsJurisdiction::Minnesota,
    UsJurisdiction::Iowa,
    UsJurisdiction::Illinois,
    UsJurisdiction::Colorado,
];

#[derive(Debug, Clone)]
pub(crate) struct NcesSelectedInputV1 {
    pub(crate) source_root: PathBuf,
    pub(crate) run_manifest_digest: String,
    pub(crate) scope: Vec<UsJurisdiction>,
    pub(crate) ccd: NcesSelectedArtifactV1,
    pub(crate) pss: NcesSelectedArtifactV1,
}

#[derive(Debug, Clone)]
pub(crate) struct NcesSelectedArtifactV1 {
    pub(crate) path: PathBuf,
    pub(crate) sha256: String,
    pub(crate) source_url: String,
    pub(crate) captured_on: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplayManifest {
    schema_revision: u32,
    month: String,
    captures: Vec<ReplayCapture>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplayCapture {
    format: ReplayFormat,
    path: PathBuf,
    url: String,
    retrieved_at: String,
    sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ReplayFormat {
    NcesCcd,
    NcesPss,
}

impl ReplayFormat {
    const fn provider(self) -> NcesArtifactProviderV1 {
        match self {
            Self::NcesCcd => NcesArtifactProviderV1::Ccd,
            Self::NcesPss => NcesArtifactProviderV1::Pss,
        }
    }
}

struct VerifiedBytes {
    bytes: Vec<u8>,
    sha256: String,
    byte_length: u64,
}

/// Verify run and input bindings before writing, then use the existing parser and replay archive.
pub(crate) fn accept(
    store: &Store,
    selection: &NcesSelectedInputV1,
) -> Result<NcesAddressRunReceiptV1> {
    let run = store
        .run_manifest()?
        .context("NCES address intake requires a bound Store run")?;
    validate_selection(selection, &run)?;
    let ccd = read_verified(&selection.source_root, &selection.ccd)?;
    let pss = read_verified(&selection.source_root, &selection.pss)?;

    // Domain references validate the provider, digest, size, URL and date before any persistent
    // generation or Store write is attempted.
    let ccd_ref = NcesArtifactReferenceV1::new(
        NcesArtifactProviderV1::Ccd,
        ccd.sha256.clone(),
        ccd.byte_length,
        selection.ccd.source_url.clone(),
        selection.ccd.captured_on.clone(),
    )?;
    let pss_ref = NcesArtifactReferenceV1::new(
        NcesArtifactProviderV1::Pss,
        pss.sha256.clone(),
        pss.byte_length,
        selection.pss.source_url.clone(),
        selection.pss.captured_on.clone(),
    )?;

    let replay_manifest = ReplayManifest {
        schema_revision: 1,
        month: "2026-10".to_string(),
        captures: vec![
            ReplayCapture {
                format: ReplayFormat::NcesCcd,
                path: PathBuf::from("ccd.csv"),
                url: selection.ccd.source_url.clone(),
                retrieved_at: selection.ccd.captured_on.clone(),
                sha256: ccd.sha256.clone(),
            },
            ReplayCapture {
                format: ReplayFormat::NcesPss,
                path: PathBuf::from("pss.csv"),
                url: selection.pss.source_url.clone(),
                retrieved_at: selection.pss.captured_on.clone(),
                sha256: pss.sha256.clone(),
            },
        ],
    };
    let manifest_bytes = serde_json::to_vec(&replay_manifest)?;
    let input_manifest_digest = sha256_hex(&manifest_bytes);

    if let Some(existing) = store.nces_address_run_v1()? {
        if existing.run_manifest_digest() != selection.run_manifest_digest
            || existing.scope() != selection.scope
            || existing.ccd() != &ccd_ref
            || existing.pss() != &pss_ref
            || existing.input_manifest_digest() != input_manifest_digest
        {
            bail!("a different immutable NCES address receipt is already bound to this Store");
        }
        return Ok(reopen(store, &selection.run_manifest_digest)?.0);
    }

    let staging = tempfile::tempdir().context("creating bounded NCES input staging area")?;
    let ccd_name = Path::new("ccd.csv");
    let pss_name = Path::new("pss.csv");
    std::fs::write(staging.path().join(ccd_name), &ccd.bytes)
        .context("staging verified CCD bytes")?;
    std::fs::write(staging.path().join(pss_name), &pss.bytes)
        .context("staging verified PSS bytes")?;
    let manifest_path = staging.path().join("manifest.json");
    std::fs::write(&manifest_path, &manifest_bytes)
        .context("writing replay manifest for verified NCES inputs")?;

    let out = store.out_dir().join("nces-address");
    if out.exists() {
        bail!("NCES address output exists without a matching immutable Store receipt");
    }
    let replay = super::replay::run(&manifest_path, &out)?;
    let frozen = super::verify_current(&out)?;
    verify_generation(
        &frozen,
        &selection.scope,
        &ccd_ref,
        &pss_ref,
        Some(replay.generation_digest()),
    )?;
    let receipt = NcesAddressRunReceiptV1::new(
        selection.run_manifest_digest.clone(),
        selection.scope.clone(),
        ccd_ref,
        pss_ref,
        input_manifest_digest,
        replay.generation_digest().to_string(),
    )?;
    store.bind_nces_address_run_v1(&receipt)?;
    Ok(receipt)
}

/// Reopen Store lineage, generated address artifacts and retained raw captures after input removal.
pub(crate) fn reopen(
    store: &Store,
    expected_run_manifest_digest: &str,
) -> Result<(NcesAddressRunReceiptV1, super::VerifiedGeneration)> {
    let run = store
        .run_manifest()?
        .context("NCES address reopen requires a bound Store run")?;
    if serialized_digest(&run)? != expected_run_manifest_digest {
        bail!("NCES address receipt was requested for a different Store run");
    }
    let receipt = store
        .nces_address_run_v1()?
        .context("NCES address run receipt is absent")?;
    if receipt.run_manifest_digest() != expected_run_manifest_digest {
        bail!("NCES address receipt run digest mismatch");
    }
    let out = store.out_dir().join("nces-address");
    let generation = super::verify_current(&out)?;
    verify_generation(
        &generation,
        receipt.scope(),
        receipt.ccd(),
        receipt.pss(),
        Some(receipt.address_generation_digest()),
    )?;
    verify_retained_inputs(&out, &receipt)?;
    Ok((receipt, generation))
}

fn validate_selection(selection: &NcesSelectedInputV1, run: &RunManifest) -> Result<()> {
    let digest = serialized_digest(run)?;
    if selection.run_manifest_digest != digest {
        bail!("selected NCES artifacts belong to a different run manifest");
    }
    if selection.scope.as_slice() != FIVE_STATE_SCOPE || selection.scope != run.jurisdictions {
        bail!("selected NCES artifacts must use the exact MI/MN/IA/IL/CO run scope");
    }
    Ok(())
}

fn read_verified(root: &Path, artifact: &NcesSelectedArtifactV1) -> Result<VerifiedBytes> {
    if artifact.path.is_absolute()
        || artifact
            .path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("NCES input path must be a contained relative path");
    }
    let root = std::fs::canonicalize(root).context("canonicalizing NCES input root")?;
    let path = root.join(&artifact.path);
    let metadata = std::fs::symlink_metadata(&path)
        .with_context(|| format!("reading NCES input metadata at {}", path.display()))?;
    if !metadata.file_type().is_file() || metadata.len() == 0 || metadata.len() > MAX_ARTIFACT_BYTES
    {
        bail!("NCES input must be a nonempty bounded regular file");
    }
    let canonical = std::fs::canonicalize(&path)?;
    if !canonical.starts_with(&root) {
        bail!("NCES input escapes its declared source root");
    }
    let file = std::fs::File::open(&canonical)?;
    let mut bytes = Vec::new();
    let capacity =
        usize::try_from(metadata.len()).context("NCES input size is not representable")?;
    bytes
        .try_reserve_exact(capacity)
        .context("reserving bounded NCES input")?;
    file.take(MAX_ARTIFACT_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)?;
    let byte_length = u64::try_from(bytes.len())?;
    if byte_length == 0 || byte_length > MAX_ARTIFACT_BYTES {
        bail!("NCES input grew beyond its byte limit");
    }
    let sha256 = sha256_hex(&bytes);
    if sha256 != artifact.sha256 {
        bail!("NCES input SHA-256 does not match its selected digest");
    }
    Ok(VerifiedBytes {
        bytes,
        sha256,
        byte_length,
    })
}

fn verify_generation(
    generation: &super::VerifiedGeneration,
    scope: &[UsJurisdiction],
    ccd: &NcesArtifactReferenceV1,
    pss: &NcesArtifactReferenceV1,
    expected_generation_digest: Option<&str>,
) -> Result<()> {
    let report: super::Report =
        serde_json::from_slice(generation.artifact("pipeline_report.json")?)?;
    if report.lanes.len() != 2 {
        bail!("NCES address generation must contain exactly two input lanes");
    }
    if expected_generation_digest.is_some_and(|digest| report.manifest_digest != digest) {
        bail!("NCES address generation digest differs from its Store receipt");
    }
    for (provider, digest) in [("nces-ccd", ccd.sha256()), ("nces-pss", pss.sha256())] {
        if !report
            .lanes
            .iter()
            .any(|lane| lane.source == provider && lane.sha256 == digest)
        {
            bail!("school-address generation does not preserve {provider} digest");
        }
    }
    let entries: Vec<SchoolDirectoryEntry> =
        serde_json::from_slice(generation.artifact("school_directory.json")?)?;
    if entries.iter().any(|entry| {
        entry
            .jurisdiction()
            .is_none_or(|jurisdiction| !scope.contains(&jurisdiction))
    }) {
        bail!("NCES address generation contains a row outside its selected run scope");
    }
    Ok(())
}

fn verify_retained_inputs(out: &Path, receipt: &NcesAddressRunReceiptV1) -> Result<()> {
    let archive = out.join("captures").join(receipt.input_manifest_digest());
    let manifest_path = archive.join("manifest.json");
    let manifest_bytes = std::fs::read(&manifest_path)
        .with_context(|| format!("reading retained NCES manifest {}", manifest_path.display()))?;
    if sha256_hex(&manifest_bytes) != receipt.input_manifest_digest()
        || !archive.join(".complete").is_file()
    {
        bail!("retained NCES input manifest is incomplete or has a digest mismatch");
    }
    let manifest: ReplayManifest = serde_json::from_slice(&manifest_bytes)?;
    if manifest.schema_revision != 1 || manifest.captures.len() != 2 {
        bail!("retained NCES input manifest has an unsupported shape");
    }
    for (capture, expected) in manifest.captures.iter().zip([
        (receipt.ccd(), ReplayFormat::NcesCcd),
        (receipt.pss(), ReplayFormat::NcesPss),
    ]) {
        let (artifact, format) = expected;
        if capture.format != format
            || capture.format.provider()
                != if format == ReplayFormat::NcesCcd {
                    receipt.ccd().provider()
                } else {
                    receipt.pss().provider()
                }
            || capture.sha256 != artifact.sha256()
            || capture.url != artifact.source_url()
            || capture.retrieved_at != artifact.captured_on()
            || capture.path.is_absolute()
            || capture.path.components().count() != 1
        {
            bail!("retained NCES capture metadata differs from its Store receipt");
        }
        let body = std::fs::read(archive.join(artifact.sha256()))?;
        if u64::try_from(body.len())? != artifact.byte_length()
            || sha256_hex(&body) != artifact.sha256()
        {
            bail!("retained NCES capture bytes differ from their Store receipt");
        }
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::model::{CensusRun, GradYear, RunManifest, SchoolYear};
    use std::collections::BTreeSet;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const CCD: &[u8] = include_bytes!("../../tests/fixtures/nces_run/ccd_synthetic.csv");
    const PSS: &[u8] = include_bytes!("../../tests/fixtures/nces_run/pss_synthetic.csv");

    const CCD_SHA256: &str = "57c07542819bb066c4ec68e20ab2eb53d32eeccd84408e457418fa0ecd37f94d";
    const PSS_SHA256: &str = "777b31b02fb7f47652049f8f6ad81a8b1f8f875aaa4ff948f173fea4cfe1adcf";

    struct Fixture {
        _root: tempfile::TempDir,
        source_root: PathBuf,
        store_path: PathBuf,
        store: Store,
        run_digest: String,
        selection: NcesSelectedInputV1,
    }

    fn fixture() -> TestResult<Fixture> {
        let root = tempfile::tempdir()?;
        let source_root = root.path().join("inputs");
        std::fs::create_dir(&source_root)?;
        std::fs::write(source_root.join("ccd.csv"), CCD)?;
        std::fs::write(source_root.join("pss.csv"), PSS)?;

        let store_path = root.path().join("store");
        let store = Store::open(&store_path)?;
        let manifest = RunManifest {
            store_identity: store.store_identity()?.as_str().to_string(),
            run: CensusRun::new(SchoolYear::new(2026).ok_or("school year")?, 1)
                .ok_or("run revision")?,
            cohort: GradYear::CO2027,
            jurisdictions: FIVE_STATE_SCOPE.to_vec(),
        };
        store.bind_run(&manifest)?;
        let run_digest = serialized_digest(&manifest)?;
        let selection = NcesSelectedInputV1 {
            source_root: source_root.clone(),
            run_manifest_digest: run_digest.clone(),
            scope: FIVE_STATE_SCOPE.to_vec(),
            ccd: NcesSelectedArtifactV1 {
                path: PathBuf::from("ccd.csv"),
                sha256: CCD_SHA256.to_string(),
                source_url: "https://fixtures.invalid/nces/ccd.csv".to_string(),
                captured_on: "2026-10-20".to_string(),
            },
            pss: NcesSelectedArtifactV1 {
                path: PathBuf::from("pss.csv"),
                sha256: PSS_SHA256.to_string(),
                source_url: "https://fixtures.invalid/nces/pss.csv".to_string(),
                captured_on: "2026-10-20".to_string(),
            },
        };
        Ok(Fixture {
            _root: root,
            source_root,
            store_path,
            store,
            run_digest,
            selection,
        })
    }

    fn assert_no_writes(fixture: &Fixture, sequence: u64) -> TestResult {
        check!(eq; fixture.store.snapshot().sequence(), sequence);
        check!(fixture
            .store
            .scan::<census_domain::model::CanonicalSchool>(census_store::Table::Schools)?
            .is_empty());
        check!(eq; fixture.store.nces_address_run_v1()?, None);
        check!(!fixture.store.out_dir().join("nces-address").exists());
        Ok(())
    }

    #[test]
    fn nces_address_run_accepts_exact_hash_pinned_five_state_inputs_and_reopens_frozen_output(
    ) -> TestResult {
        let fixture = fixture()?;
        check!(eq; sha256_hex(CCD), CCD_SHA256);
        check!(eq; sha256_hex(PSS), PSS_SHA256);
        let receipt = accept(&fixture.store, &fixture.selection)?;
        check!(eq; receipt.run_manifest_digest(), fixture.run_digest);
        check!(eq; receipt.scope(), FIVE_STATE_SCOPE);
        check!(eq; receipt.ccd().sha256(), CCD_SHA256);
        check!(eq; receipt.pss().sha256(), PSS_SHA256);
        check!(eq; fixture.store.nces_address_run_v1()?, Some(receipt.clone()));
        let committed_sequence = fixture.store.snapshot().sequence();
        check!(eq; accept(&fixture.store, &fixture.selection)?, receipt);
        check!(eq; fixture.store.snapshot().sequence(), committed_sequence);

        std::fs::remove_file(fixture.source_root.join("ccd.csv"))?;
        std::fs::remove_file(fixture.source_root.join("pss.csv"))?;
        let reopened_store = Store::open(&fixture.store_path)?;
        let (reopened_receipt, generation) = reopen(&reopened_store, &fixture.run_digest)?;
        check!(eq; reopened_receipt, receipt);
        let report: super::super::Report =
            serde_json::from_slice(generation.artifact("pipeline_report.json")?)?;
        check!(eq; report.lanes.len(), 2);
        check!(eq; report.manifest_digest, reopened_receipt.address_generation_digest());
        let entries: Vec<SchoolDirectoryEntry> =
            serde_json::from_slice(generation.artifact("school_directory.json")?)?;
        check!(eq; entries.len(), 10);
        check!(entries.iter().all(|entry| entry.address().is_some()));
        let states: BTreeSet<UsJurisdiction> = entries
            .iter()
            .map(|entry| entry.jurisdiction().ok_or("entry has no state"))
            .collect::<std::result::Result<_, _>>()?;
        check!(eq; states, FIVE_STATE_SCOPE.into_iter().collect());
        Ok(())
    }

    #[test]
    fn nces_address_run_missing_file_refuses_before_store_or_output_writes() -> TestResult {
        let mut fixture = fixture()?;
        fixture.selection.ccd.path = PathBuf::from("missing.csv");
        let sequence = fixture.store.snapshot().sequence();
        check!(accept(&fixture.store, &fixture.selection).is_err());
        assert_no_writes(&fixture, sequence)
    }

    #[test]
    fn nces_address_run_wrong_run_digest_refuses_before_store_or_output_writes() -> TestResult {
        let mut fixture = fixture()?;
        fixture.selection.run_manifest_digest = "0".repeat(64);
        let sequence = fixture.store.snapshot().sequence();
        check!(accept(&fixture.store, &fixture.selection).is_err());
        assert_no_writes(&fixture, sequence)
    }

    #[test]
    fn nces_address_run_hash_mismatch_refuses_before_store_or_output_writes() -> TestResult {
        let mut fixture = fixture()?;
        fixture.selection.pss.sha256 = "0".repeat(64);
        let sequence = fixture.store.snapshot().sequence();
        check!(accept(&fixture.store, &fixture.selection).is_err());
        assert_no_writes(&fixture, sequence)
    }

    #[test]
    fn nces_address_run_changed_provenance_refuses_without_rewriting_existing_generation(
    ) -> TestResult {
        let mut fixture = fixture()?;
        let receipt = accept(&fixture.store, &fixture.selection)?;
        let out = fixture.store.out_dir().join("nces-address");
        let current = std::fs::read_link(out.join("current"))?;
        let sequence = fixture.store.snapshot().sequence();
        fixture.selection.ccd.source_url =
            "https://fixtures.invalid/nces/changed-ccd.csv".to_string();
        check!(accept(&fixture.store, &fixture.selection).is_err());
        check!(eq; fixture.store.snapshot().sequence(), sequence);
        check!(eq; fixture.store.nces_address_run_v1()?, Some(receipt));
        check!(eq; std::fs::read_link(out.join("current"))?, current);
        Ok(())
    }
}
