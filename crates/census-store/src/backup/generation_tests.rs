use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::super::manifest::write_manifest;
use super::super::{Manifest, MANIFEST_PATH, MANIFEST_VERSION, STAGING_PREFIX};
use super::{destination_prefix, parent_of, promote, recover, Generation, PromotionStep, Recovered};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const OLD_AT: &str = "2026-10-01T00:00:00+00:00";
const NEW_AT: &str = "2026-10-02T00:00:00+00:00";
const LATEST_AT: &str = "2026-10-03T00:00:00+00:00";

fn manifest(written_at: &str) -> Manifest {
    Manifest {
        version: MANIFEST_VERSION,
        written_at: written_at.to_string(),
        files: Vec::new(),
        tables: BTreeMap::new(),
    }
}

fn generation_at(path: &Path, marker: &str, written_at: &str) -> TestResult {
    fs::create_dir_all(path)?;
    fs::write(path.join(marker), marker.as_bytes())?;
    write_manifest(path, &manifest(written_at))?;
    Ok(())
}

fn displace(to: &Path, token: &str) -> TestResult<PathBuf> {
    let prefix = destination_prefix(STAGING_PREFIX, to);
    let trash = parent_of(to).join(format!("{prefix}old.{token}"));
    fs::rename(to, &trash)?;
    Ok(trash)
}

fn abandoned_staging(
    to: &Path,
    token: &str,
    marker: &str,
    written_at: &str,
) -> TestResult<PathBuf> {
    let prefix = destination_prefix(STAGING_PREFIX, to);
    let staging = parent_of(to).join(format!("{prefix}{token}"));
    generation_at(&staging, marker, written_at)?;
    Ok(staging)
}

fn marker(path: &Path) -> TestResult<Vec<String>> {
    let mut found = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name != MANIFEST_PATH {
                found.push(name);
            }
        }
    }
    found.sort();
    Ok(found)
}

fn leftovers(parent: &Path) -> TestResult<Vec<String>> {
    let mut left = Vec::new();
    for entry in fs::read_dir(parent)? {
        let name = entry?.file_name().to_string_lossy().to_string();
        if name.starts_with(STAGING_PREFIX) {
            left.push(name);
        }
    }
    left.sort();
    Ok(left)
}

fn crash_error() -> std::io::Error {
    std::io::Error::other("simulated crash")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Crash {
    BeforeExchange,
    AfterExchange,
}

#[test]
fn every_interrupted_promotion_step_leaves_a_reachable_generation() -> TestResult {
    for crash in [Crash::BeforeExchange, Crash::AfterExchange] {
        let dir = tempfile::tempdir()?;
        let to = dir.path().join("backup");
        generation_at(&to, "old-marker", OLD_AT)?;
        let generation = Generation::create(&to, STAGING_PREFIX)?;
        fs::write(generation.path().join("new-marker"), b"new-marker")?;
        write_manifest(generation.path(), &manifest(NEW_AT))?;
        let result = generation.publish_with(&mut |step, from: &Path, rename_to: &Path| match (
            crash, step,
        ) {
            (Crash::BeforeExchange, PromotionStep::Exchange) => Err(crash_error()),
            (Crash::AfterExchange, PromotionStep::Exchange) => {
                promote(step, from, rename_to)?;
                Err(crash_error())
            }
            _ => promote(step, from, rename_to),
        });
        check!(
            result.is_err(),
            "the simulated crash must surface as a refusal: {crash:?}"
        );
        recover(&to, STAGING_PREFIX)?;
        check!(
            to.join(MANIFEST_PATH).is_file(),
            "the canonical destination must hold a published generation after {crash:?}"
        );
        let found = marker(&to)?;
        check!(
            found.len() == 1
                && (found.first().map(String::as_str) == Some("old-marker")
                    || found.first().map(String::as_str) == Some("new-marker")),
            "the destination must hold the old or the new generation after {crash:?}: {found:?}"
        );
        check!(
            eq;
            leftovers(dir.path())?,
            Vec::<String>::new(),
            "no staging or displaced generation may survive recovery after {crash:?}"
        );
        check!(
            eq;
            recover(&to, STAGING_PREFIX)?,
            Recovered::default(),
            "a recovered destination has nothing left to recover after {crash:?}"
        );
    }
    Ok(())
}

#[test]
fn the_canonical_generation_never_disappears_during_replacement() -> TestResult {
    let dir = tempfile::tempdir()?;
    let to = dir.path().join("backup");
    generation_at(&to, "old-marker", OLD_AT)?;
    let generation = Generation::create(&to, STAGING_PREFIX)?;
    generation_at(generation.path(), "new-marker", NEW_AT)?;
    let mut absent = false;
    let result = generation.publish_with(&mut |step, from, target| {
        absent |= !to.join(MANIFEST_PATH).is_file();
        if step == PromotionStep::Exchange {
            return Err(crash_error());
        }
        promote(step, from, target)
    });
    check!(result.is_err());
    check!(
        !absent,
        "the canonical path disappeared between promotion steps"
    );
    check!(eq; marker(&to)?, vec!["old-marker".to_string()]);
    Ok(())
}

#[test]
fn a_crash_that_displaced_the_destination_rolls_the_old_generation_back() -> TestResult {
    let dir = tempfile::tempdir()?;
    let to = dir.path().join("backup");
    generation_at(&to, "old-marker", OLD_AT)?;
    let trash = displace(&to, "crashed")?;
    let staging = abandoned_staging(&to, "crashed", "new-marker", NEW_AT)?;
    check!(
        !to.exists(),
        "the crash window leaves the canonical destination absent"
    );
    let recovered = recover(&to, STAGING_PREFIX)?;
    check!(
        recovered.restored,
        "the displaced generation must come back"
    );
    check!(eq; recovered.staging_removed, 1);
    check!(eq; recovered.trashes_removed, 0);
    check!(eq; marker(&to)?, vec!["old-marker".to_string()]);
    check!(to.join(MANIFEST_PATH).is_file());
    check!(!trash.exists() && !staging.exists());
    check!(eq; leftovers(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn a_crash_after_the_promotion_rename_keeps_the_new_generation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let to = dir.path().join("backup");
    generation_at(&to, "old-marker", OLD_AT)?;
    let trash = displace(&to, "crashed")?;
    let staging = abandoned_staging(&to, "crash-window", "new-marker", NEW_AT)?;
    fs::rename(&staging, &to)?;
    let recovered = recover(&to, STAGING_PREFIX)?;
    check!(
        !recovered.restored,
        "a promoted destination is not rolled back"
    );
    check!(eq; recovered.trashes_removed, 1);
    check!(eq; recovered.staging_removed, 0);
    check!(eq; marker(&to)?, vec!["new-marker".to_string()]);
    check!(!trash.exists());
    check!(eq; leftovers(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn recovery_restores_the_newest_displaced_generation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let to = dir.path().join("backup");
    generation_at(&to, "first-marker", OLD_AT)?;
    let first = displace(&to, "older")?;
    generation_at(&to, "second-marker", LATEST_AT)?;
    let second = displace(&to, "newer")?;
    check!(!to.exists());
    let recovered = recover(&to, STAGING_PREFIX)?;
    check!(recovered.restored);
    check!(eq; recovered.trashes_removed, 1);
    check!(eq; marker(&to)?, vec!["second-marker".to_string()]);
    check!(!first.exists() && !second.exists());
    Ok(())
}

#[test]
fn recovery_prefers_a_displaced_generation_with_a_readable_manifest() -> TestResult {
    let dir = tempfile::tempdir()?;
    let to = dir.path().join("backup");
    generation_at(&to, "readable-marker", OLD_AT)?;
    let readable = displace(&to, "zzz")?;
    generation_at(&to, "damaged-marker", LATEST_AT)?;
    let damaged = displace(&to, "aaa")?;
    fs::write(damaged.join(MANIFEST_PATH), b"not a manifest")?;
    check!(!to.exists());
    let recovered = recover(&to, STAGING_PREFIX)?;
    check!(recovered.restored);
    check!(
        eq;
        marker(&to)?,
        vec!["readable-marker".to_string()],
        "the generation with a readable manifest is the one that comes back"
    );
    check!(!readable.exists() && !damaged.exists());
    Ok(())
}

#[test]
fn recovery_removes_abandoned_staging_generations() -> TestResult {
    let dir = tempfile::tempdir()?;
    let to = dir.path().join("backup");
    generation_at(&to, "live-marker", OLD_AT)?;
    let staging = abandoned_staging(&to, "abandoned", "dead-marker", NEW_AT)?;
    let recovered = recover(&to, STAGING_PREFIX)?;
    check!(!recovered.restored);
    check!(eq; recovered.staging_removed, 1);
    check!(eq; recovered.trashes_removed, 0);
    check!(eq; marker(&to)?, vec!["live-marker".to_string()]);
    check!(!staging.exists());
    Ok(())
}

#[test]
fn recovery_keeps_displaced_generations_when_the_destination_is_not_a_generation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let to = dir.path().join("backup");
    generation_at(&to, "displaced-marker", OLD_AT)?;
    let trash = displace(&to, "kept")?;
    fs::create_dir_all(&to)?;
    fs::write(to.join("foreign.txt"), b"not ours")?;
    let staging = abandoned_staging(&to, "abandoned", "dead-marker", NEW_AT)?;
    let recovered = recover(&to, STAGING_PREFIX)?;
    check!(
        !recovered.restored,
        "a destination that exists is never replaced"
    );
    check!(eq; recovered.staging_removed, 1);
    check!(eq; recovered.trashes_removed, 0);
    check!(
        trash.join(MANIFEST_PATH).is_file(),
        "the displaced generation is kept"
    );
    check!(eq; fs::read_to_string(to.join("foreign.txt"))?, "not ours".to_string());
    check!(!staging.exists());
    Ok(())
}

#[test]
fn recovery_discards_an_empty_displaced_directory_beside_a_destination() -> TestResult {
    let dir = tempfile::tempdir()?;
    let to = dir.path().join("restored");
    fs::create_dir_all(&to)?;
    fs::write(to.join("fjall-stand-in"), b"store")?;
    let prefix = destination_prefix(STAGING_PREFIX, &to);
    let empty = parent_of(&to).join(format!("{prefix}old.empty"));
    fs::create_dir_all(&empty)?;
    let recovered = recover(&to, STAGING_PREFIX)?;
    check!(!recovered.restored);
    check!(eq; recovered.trashes_removed, 1);
    check!(
        !empty.exists(),
        "an empty displaced directory holds nothing to keep"
    );
    check!(to.join("fjall-stand-in").is_file());
    Ok(())
}

#[test]
fn a_failed_promotion_rename_puts_the_displaced_generation_back() -> TestResult {
    let dir = tempfile::tempdir()?;
    let to = dir.path().join("backup");
    generation_at(&to, "old-marker", OLD_AT)?;
    let generation = Generation::create(&to, STAGING_PREFIX)?;
    fs::write(generation.path().join("new-marker"), b"new-marker")?;
    write_manifest(generation.path(), &manifest(NEW_AT))?;
    let result = generation.publish_with(&mut |step, from: &Path, rename_to: &Path| match step {
        PromotionStep::Exchange => Err(crash_error()),
        _ => promote(step, from, rename_to),
    });
    let error = match result {
        Err(error) => error.to_string(),
        Ok(()) => return Err("the failed promotion must be refused".into()),
    };
    check!(error.contains("simulated crash"), "{error}");
    check!(
        eq;
        marker(&to)?,
        vec!["old-marker".to_string()],
        "the destination must hold the displaced generation again"
    );
    check!(eq; leftovers(dir.path())?, Vec::<String>::new());
    check!(eq; recover(&to, STAGING_PREFIX)?, Recovered::default());
    Ok(())
}

#[test]
fn a_publish_that_succeeds_leaves_only_the_destination() -> TestResult {
    let dir = tempfile::tempdir()?;
    let to = dir.path().join("backup");
    generation_at(&to, "old-marker", OLD_AT)?;
    let generation = Generation::create(&to, STAGING_PREFIX)?;
    fs::write(generation.path().join("new-marker"), b"new-marker")?;
    write_manifest(generation.path(), &manifest(NEW_AT))?;
    generation.publish()?;
    check!(eq; marker(&to)?, vec!["new-marker".to_string()]);
    check!(eq; leftovers(dir.path())?, Vec::<String>::new());
    check!(eq; recover(&to, STAGING_PREFIX)?, Recovered::default());
    Ok(())
}

#[test]
fn process_death_at_the_exchange_boundary_keeps_one_canonical_generation() -> TestResult {
    const CHILD: &str = "CENSUS_BACKUP_EXCHANGE_CHILD";
    if let Some(to) = std::env::var_os(CHILD) {
        let to = PathBuf::from(to);
        let generation = Generation::create(&to, STAGING_PREFIX)?;
        generation_at(generation.path(), "new-marker", NEW_AT)?;
        generation.publish_with(&mut |step, from, target| {
            if std::env::var_os("CENSUS_BACKUP_BEFORE_EXCHANGE").is_some() {
                std::process::exit(73);
            }
            promote(step, from, target)?;
            std::process::exit(73);
        })?;
        return Err("the crash child returned without exiting".into());
    }
    for before in [true, false] {
        let dir = tempfile::tempdir()?;
        let to = dir.path().join("backup");
        generation_at(&to, "old-marker", OLD_AT)?;
        let mut command = std::process::Command::new(std::env::current_exe()?);
        command.args(["--exact", "backup::generation::generation_tests::process_death_at_the_exchange_boundary_keeps_one_canonical_generation", "--nocapture"]);
        command.env(CHILD, &to);
        if before {
            command.env("CENSUS_BACKUP_BEFORE_EXCHANGE", "1");
        }
        check!(eq; command.status()?.code(), Some(73));
        check!(to.join(MANIFEST_PATH).is_file());
        let expected = if before { "old-marker" } else { "new-marker" };
        check!(eq; marker(&to)?, vec![expected.to_string()]);
        recover(&to, STAGING_PREFIX)?;
        check!(eq; marker(&to)?, vec![expected.to_string()]);
        check!(eq; leftovers(dir.path())?, Vec::<String>::new());
    }
    Ok(())
}


#[test]
fn sibling_destinations_share_a_parent_without_crosstalk() -> TestResult {
    let dir = tempfile::tempdir()?;
    let to_a = dir.path().join("backup-alpha");
    let to_b = dir.path().join("backup-beta");
    generation_at(&to_a, "alpha-old", OLD_AT)?;
    generation_at(&to_b, "beta-old", OLD_AT)?;
    let prefix_a = destination_prefix(STAGING_PREFIX, &to_a);
    let prefix_b = destination_prefix(STAGING_PREFIX, &to_b);
    check!(prefix_a != prefix_b, "destination prefixes must differ");
    let staging_a = abandoned_staging_with_prefix(&to_a, "a-stage", "alpha-new", NEW_AT, &prefix_a)?;
    let staging_b = abandoned_staging_with_prefix(&to_b, "b-stage", "beta-new", NEW_AT, &prefix_b)?;
    let recovered_a = recover(&to_a, STAGING_PREFIX)?;
    check!(eq; recovered_a.staging_removed, 1);
    check!(eq; recovered_a.trashes_removed, 0);
    check!(eq; marker(&to_a)?, vec!["alpha-old".to_string()]);
    check!(
        staging_b.exists(),
        "recovering A must not delete B's staging directory"
    );
    let recovered_b = recover(&to_b, STAGING_PREFIX)?;
    check!(eq; recovered_b.staging_removed, 1);
    check!(eq; recovered_b.trashes_removed, 0);
    check!(eq; marker(&to_b)?, vec!["beta-old".to_string()]);
    check!(!staging_a.exists());
    Ok(())
}

#[test]
fn recovering_one_sibling_preserves_the_others_only_old_generation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let to_a = dir.path().join("backup-alpha");
    let to_b = dir.path().join("backup-beta");
    generation_at(&to_a, "alpha-old", OLD_AT)?;
    generation_at(&to_b, "beta-old", OLD_AT)?;
    let prefix_a = destination_prefix(STAGING_PREFIX, &to_a);
    let prefix_b = destination_prefix(STAGING_PREFIX, &to_b);
    let trash_a = displace_with_prefix(&to_a, "a-old", &prefix_a)?;
    generation_at(&to_a, "alpha-new", NEW_AT)?;
    check!(trash_a.exists());
    let recovered_a = recover(&to_a, STAGING_PREFIX)?;
    check!(eq; recovered_a.trashes_removed, 1);
    check!(eq; recovered_a.staging_removed, 0);
    check!(
        to_b.join(MANIFEST_PATH).is_file(),
        "recovery of A must not affect B's published generation"
    );
    let recovered_b = recover(&to_b, STAGING_PREFIX)?;
    check!(eq; recovered_b.trashes_removed, 0);
    check!(eq; recovered_b.staging_removed, 0);
    check!(eq; recovered_b.restored, false);
    check!(eq; marker(&to_b)?, vec!["beta-old".to_string()]);
    Ok(())
}

#[test]
fn recovery_preserves_an_artifact_without_a_known_owner() -> TestResult {
    let dir = tempfile::tempdir()?;
    let to = dir.path().join("backup");
    generation_at(&to, "live", OLD_AT)?;
    let orphan = dir.path().join("foreign-staging");
    fs::create_dir_all(&orphan)?;
    fs::write(orphan.join("foreign.txt"), b"foreign")?;
    let recovered = recover(&to, STAGING_PREFIX)?;
    check!(eq; recovered.staging_removed, 0);
    check!(eq; recovered.trashes_removed, 0);
    check!(
        orphan.exists(),
        "an artifact with no known owner must be preserved"
    );
    check!(orphan.join("foreign.txt").is_file());
    Ok(())
}

fn abandoned_staging_with_prefix(
    to: &Path,
    token: &str,
    marker: &str,
    written_at: &str,
    prefix: &str,
) -> TestResult<PathBuf> {
    let staging = parent_of(to).join(format!("{prefix}{token}"));
    generation_at(&staging, marker, written_at)?;
    Ok(staging)
}

fn displace_with_prefix(to: &Path, token: &str, prefix: &str) -> TestResult<PathBuf> {
    let trash = parent_of(to).join(format!("{prefix}old.{token}"));
    fs::rename(to, &trash)?;
    Ok(trash)
}
