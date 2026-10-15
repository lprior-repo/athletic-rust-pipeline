use anyhow::{ensure, Context, Result};
use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

pub const VERSION: u32 = 1;
pub const DAY: &str = "2026-05-01";
pub const MANIFEST_LIMIT: u64 = 65_536;
pub const EVIDENCE_OBJECT_LIMIT: u64 = 1_000_000;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, ValueEnum, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    AllSelected,
    Mixed,
}

#[derive(Clone, Debug, Args, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[arg(long, default_value_t = 1000)]
    pub athletes: u64,
    #[arg(long, default_value_t = 4)]
    pub evidence_width: u32,
    #[arg(long, default_value_t = 256)]
    pub evidence_bytes: usize,
    #[arg(long, value_enum, default_value_t = Shape::AllSelected)]
    pub shape: Shape,
}

impl Config {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            (1..=1_000_000).contains(&self.athletes),
            "athletes outside 1..=1000000"
        );
        ensure!(
            (1..=64).contains(&self.evidence_width),
            "evidence width outside 1..=64"
        );
        ensure!(
            (1..=16_384).contains(&self.evidence_bytes),
            "evidence bytes outside 1..=16384"
        );
        ensure!(
            u64::from(self.evidence_width)
                .checked_mul(u64::try_from(self.evidence_bytes)?)
                .context("evidence note payload overflow")?
                <= 512 * 1024,
            "evidence note payload exceeds 512 KiB per record"
        );
        let objects = self.evidence_objects()?;
        ensure!(
            objects <= EVIDENCE_OBJECT_LIMIT,
            "synthetic evidence object count exceeds {EVIDENCE_OBJECT_LIMIT}"
        );
        let bytes = objects
            .checked_mul(u64::try_from(self.evidence_bytes)?)
            .context("fixture size overflow")?;
        ensure!(
            bytes <= 8 * 1024 * 1024 * 1024,
            "synthetic note payload exceeds 8 GiB"
        );
        ensure!(
            self.shape != Shape::Mixed || self.athletes <= 64,
            "mixed correctness shape limited to 64 base athletes"
        );
        Ok(())
    }

    fn evidence_objects(&self) -> Result<u64> {
        let counts = Counts::of(self)?;
        let additional = match self.shape {
            Shape::AllSelected => 1_u64,
            Shape::Mixed => 2_u64,
        };
        counts
            .stored_athletes
            .checked_add(counts.performances)
            .and_then(|n| n.checked_mul(u64::from(self.evidence_width)))
            .and_then(|n| n.checked_add(additional))
            .context("fixture evidence object count overflow")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Counts {
    pub stored_athletes: u64,
    pub selected_athletes: u64,
    pub performances: u64,
    pub selected_bests: u64,
    pub aliases: u64,
    pub outside: u64,
    pub identity_only: u64,
}

impl Counts {
    pub fn of(config: &Config) -> Result<Self> {
        let mixed = u64::from(config.shape == Shape::Mixed);
        Ok(Self {
            stored_athletes: 3u64
                .checked_mul(mixed)
                .context("stored athletes overflow")?
                .checked_add(config.athletes)
                .context("stored athletes add overflow")?,
            selected_athletes: config
                .athletes
                .checked_add(mixed)
                .context("selected athletes overflow")?,
            performances: config.athletes,
            selected_bests: config.athletes,
            aliases: mixed,
            outside: mixed,
            identity_only: mixed,
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    pub synthetic_only: bool,
    pub config: Config,
    pub counts: Counts,
    pub tables_sha256: String,
    pub fixture_sha256: String,
}

impl Manifest {
    pub fn digest(&self) -> Result<String> {
        Ok(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(
                self.version,
                self.synthetic_only,
                &self.config,
                &self.counts,
                &self.tables_sha256,
            ))?)
        ))
    }
    pub fn read(root: &Path) -> Result<Self> {
        ensure!(
            root.is_dir() && !root.is_symlink(),
            "fixture root must be a real directory"
        );
        let path = root.join("fixture.json");
        ensure!(!path.is_symlink(), "fixture manifest symlink refused");
        let bytes = bounded_read(&path, MANIFEST_LIMIT)?;
        let manifest: Self = serde_json::from_slice(&bytes)?;
        manifest.config.validate()?;
        ensure!(
            manifest.version == VERSION && manifest.synthetic_only,
            "unsupported/non-synthetic fixture"
        );
        ensure!(
            manifest.counts == Counts::of(&manifest.config)?,
            "fixture count contract mismatch"
        );
        ensure!(
            manifest.fixture_sha256 == manifest.digest()?,
            "fixture manifest digest mismatch"
        );
        ensure!(
            root.join("store").is_dir() && !root.join("store").is_symlink(),
            "fixture store missing/symlinked"
        );
        Ok(manifest)
    }
}

pub fn bounded_read(path: &Path, max: u64) -> Result<Vec<u8>> {
    let probe = max.checked_add(1).context("bounded read limit overflow")?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(probe)
        .read_to_end(&mut bytes)?;
    ensure!(
        u64::try_from(bytes.len())? <= max,
        "file exceeds bounded read: {}",
        path.display()
    );
    Ok(bytes)
}

pub fn file_digest(path: &Path) -> Result<String> {
    let mut input = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65_536];
    loop {
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(buffer.get(..n).context("digest read extent")?);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

pub fn fresh_output(root: &Path, out: &Path) -> Result<()> {
    let root = root.canonicalize()?;
    let parent = out
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .map_or(Path::new("."), core::convert::identity)
        .canonicalize()?;
    ensure!(
        !parent.starts_with(&root),
        "measurement output must be outside the fixture tree"
    );
    std::fs::create_dir(out)?;
    Ok(())
}
