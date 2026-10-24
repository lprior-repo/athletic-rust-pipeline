use super::{objects, InitialObjects};
use crate::export::proof_objects;
use crate::report::{ReportError, ReportResult};
use std::fs;
use std::path::{Path, PathBuf};

mod bounds;
#[cfg(unix)]
mod links;
mod mutation;
mod shape;
mod union;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Fixture {
    root: tempfile::TempDir,
    directory: PathBuf,
}

impl Fixture {
    fn new() -> TestResult<Self> {
        let root = tempfile::tempdir()?;
        let directory = root.path().join("proofs");
        fs::create_dir(&directory)?;
        fs::write(root.path().join("sentinel"), b"unrelated retained bytes")?;
        Ok(Self { root, directory })
    }

    fn install(&self, bytes: &[u8]) -> TestResult<objects::Reference> {
        Ok(objects::Reference {
            digest: proof_objects::install(&self.directory, bytes)?,
            bytes: u64::try_from(bytes.len())?,
        })
    }

    fn object(&self, reference: &objects::Reference, expected: &[u8]) -> TestResult {
        let path = self.directory.join(&reference.digest);
        check!(fs::symlink_metadata(&path)?.file_type().is_file());
        check!(eq; fs::read(&path)?.as_slice(), expected);
        Ok(())
    }

    fn sentinel(&self) -> TestResult {
        check!(eq; fs::read(self.root.path().join("sentinel"))?, b"unrelated retained bytes");
        Ok(())
    }
}

fn invariant<T>(result: ReportResult<T>, expected: &str) -> TestResult {
    match result {
        Err(ReportError::Invariant { detail }) => check!(eq; detail, expected),
        Err(error) => return Err(format!("expected invariant {expected:?}, got {error:?}").into()),
        Ok(_) => return Err(format!("expected invariant {expected:?}, got success").into()),
    }
    Ok(())
}

fn corrupt<T>(result: ReportResult<T>, expected_path: &Path) -> TestResult {
    match result {
        Err(ReportError::ArtifactCorrupt { path, detail }) => {
            check!(eq; path, expected_path);
            check!(eq; detail, "the frozen object does not hash to its name");
        }
        Err(error) => return Err(format!("expected checksum corruption, got {error:?}").into()),
        Ok(_) => return Err("checksum corruption was accepted".into()),
    }
    Ok(())
}

#[test]
fn fresh_accepts_empty_and_refuses_preexisting_valid_proof_without_mutation() -> TestResult {
    let fixture = Fixture::new()?;
    let policy = InitialObjects::fresh();
    policy.require_initial(&fixture.directory)?;
    check!(fs::read_dir(&fixture.directory)?
        .next()
        .transpose()?
        .is_none());
    let reference = fixture.install(b"existing valid proof")?;
    invariant(
        policy.require_initial(&fixture.directory),
        "selected proof capture requires an empty owned directory",
    )?;
    fixture.object(&reference, b"existing valid proof")?;
    fixture.sentinel()
}

#[test]
fn checked_recovery_accepts_empty_without_weakening_ordinary_inventory() -> TestResult {
    let fixture = Fixture::new()?;
    let policy = InitialObjects::checked_recovery(&fixture.directory)?;
    policy.require_initial(&fixture.directory)?;
    policy.require_with_roots(&fixture.directory, &[])?;
    check!(objects::initial_inventory(&fixture.directory)?.is_empty());
    invariant(
        objects::inventory(&fixture.directory),
        "selected proof inventory has an invalid object count",
    )?;
    check!(fs::read_dir(&fixture.directory)?
        .next()
        .transpose()?
        .is_none());
    fixture.sentinel()
}

#[test]
fn checked_recovery_accepts_valid_proofs_and_identical_installer_reuse() -> TestResult {
    let fixture = Fixture::new()?;
    let first = fixture.install(b"first retained proof")?;
    let second = fixture.install(b"second retained proof")?;
    let policy = InitialObjects::checked_recovery(&fixture.directory)?;
    policy.require_initial(&fixture.directory)?;
    check!(eq; fixture.install(b"first retained proof")?, first);
    policy.require_initial(&fixture.directory)?;
    check!(eq; objects::inventory(&fixture.directory)?.len(), 2);
    fixture.object(&first, b"first retained proof")?;
    fixture.object(&second, b"second retained proof")?;
    fixture.sentinel()
}

#[test]
fn checked_recovery_refuses_distinct_directory_even_with_identical_inventory() -> TestResult {
    let original = Fixture::new()?;
    let other = Fixture::new()?;
    let first = original.install(b"same valid object")?;
    check!(eq; other.install(b"same valid object")?, first);
    check!(ne; original.directory, other.directory);
    let policy = InitialObjects::checked_recovery(&original.directory)?;
    invariant(
        policy.require_initial(&other.directory),
        "initial proof directory binding mismatch",
    )?;
    invariant(
        policy.require_with_roots(&other.directory, &[]),
        "initial proof directory binding mismatch",
    )?;
    policy.require_initial(&original.directory)?;
    original.object(&first, b"same valid object")?;
    other.object(&first, b"same valid object")?;
    original.sentinel()?;
    other.sentinel()
}

#[test]
fn checked_recovery_refuses_checksum_corruption_during_initial_admission() -> TestResult {
    let fixture = Fixture::new()?;
    let original = b"genuine retained proof";
    let reference = fixture.install(original)?;
    let changed = vec![b'x'; original.len()];
    check!(ne; changed.as_slice(), original);
    check!(eq; u64::try_from(changed.len())?, reference.bytes);
    let path = fixture.directory.join(&reference.digest);
    fs::write(&path, &changed)?;
    corrupt(InitialObjects::checked_recovery(&fixture.directory), &path)?;
    fixture.object(&reference, &changed)?;
    fixture.sentinel()
}
