use super::*;
use census_domain::model::StoreIdentity;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

mod inventory;
mod paths;
mod tests;

pub(super) struct Prepared {
    destination: PathBuf,
    producer_commit: String,
    producer_tree: String,
    store_identity: StoreIdentity,
    snapshot_sequence: u64,
    tables_digest: String,
    inventory: Value,
}

pub(super) fn prepare(store: &Store, sources: &Sources) -> TestResult<Option<Prepared>> {
    let Some(destination) = std::env::var_os("CENSUS_DOCUMENTARY_HANDOFF_DIR") else {
        return Ok(None);
    };
    let destination = paths::destination(Path::new(&destination))?;
    let producer_commit = revision("CENSUS_DOCUMENTARY_PRODUCER_COMMIT")?;
    let producer_tree = revision("CENSUS_DOCUMENTARY_PRODUCER_TREE")?;
    let snapshot = store.snapshot();
    Ok(Some(Prepared {
        destination,
        producer_commit,
        producer_tree,
        store_identity: snapshot.store_identity()?,
        snapshot_sequence: snapshot.sequence(),
        tables_digest: snapshot.tables_digest(&Table::ALL)?,
        inventory: report::physical_inventory(store, sources)?,
    }))
}

pub(super) fn finish(prepared: Prepared, directory: tempfile::TempDir) -> TestResult {
    let kept = directory.keep();
    eprintln!(
        "GENUINE_DOCUMENTARY_HANDOFF kept_directory={}",
        kept.display()
    );
    let kept = paths::kept(&kept)?;
    let store_path = kept.join("store");
    let destination = paths::destination(&prepared.destination)?;
    if destination.starts_with(&kept) {
        return Err("handoff destination must be outside the kept directory".into());
    }
    let files = inventory::read(&store_path)?;
    let manifest = json!({
        "schema":"genuine-documentary-report-handoff/v1",
        "producer_commit":prepared.producer_commit,
        "producer_tree":prepared.producer_tree,
        "store_path":paths::text(&store_path)?,
        "store_identity":prepared.store_identity,
        "snapshot_sequence":prepared.snapshot_sequence,
        "tables_digest":prepared.tables_digest,
        "inventory":prepared.inventory,
        "files":files,
    });
    write_manifest(&destination, &manifest)?;
    eprintln!(
        "GENUINE_DOCUMENTARY_HANDOFF manifest={}",
        destination.join("handoff.json").display()
    );
    Ok(())
}

fn revision(name: &str) -> TestResult<String> {
    let value = std::env::var(name)?;
    validate_revision(&value)?;
    Ok(value)
}

fn validate_revision(value: &str) -> TestResult {
    if value.len() != 40
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(
            "handoff producer revision must be exactly 40 lowercase hexadecimal bytes".into(),
        );
    }
    Ok(())
}

fn write_manifest(destination: &Path, manifest: &Value) -> TestResult {
    let bytes = serialized_bytes(manifest)?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("handoff manifest exceeds the 32 MiB receipt bound".into());
    }
    std::fs::create_dir(destination)?;
    let directory = rustix::fs::open(
        destination,
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::DIRECTORY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )?;
    let output = rustix::fs::openat(
        &directory,
        "handoff.json",
        rustix::fs::OFlags::WRONLY
            | rustix::fs::OFlags::CREATE
            | rustix::fs::OFlags::EXCL
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )?;
    let mut output = File::from(output);
    output.write_all(&bytes)?;
    output.sync_all()?;
    File::from(directory).sync_all()?;
    Ok(())
}
