use super::*;
use crate::school_address::manifest::{Inputs, Manifest};
use crate::school_address::{CorpusReport, Report};
use census_domain::school_directory::{
    Baseline, IdentifiedKey, NcesSchoolId, ScheduleLedger, ScheduleSource, SchoolDirectoryEntry,
    SchoolName, SourceLabel,
};
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn payload(month: &str) -> TestResult<(BTreeMap<String, Vec<u8>>, Manifest)> {
    let entries = vec![SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("010000500870")?),
        SourceLabel::Ccd,
        Some(SchoolName::parse(month)?),
    )];
    let mut ledger = ScheduleLedger::new();
    ledger.record(ScheduleSource::Ccd, month.parse()?);
    let mut files = BTreeMap::from([
        (
            "school_directory.json".to_string(),
            super::super::export::json(&entries)?,
        ),
        (
            "school_directory.csv".to_string(),
            super::super::export::entries_csv(&entries)?,
        ),
        (
            "baseline.json".to_string(),
            super::super::export::json(&Baseline::new(entries))?,
        ),
        (
            "update_ledger.json".to_string(),
            super::super::export::json(&ledger)?,
        ),
    ]);
    let manifest = Manifest::new(
        Inputs {
            lanes: Vec::new(),
            baseline: None,
            ledger: None,
        },
        Some(month.to_string()),
        &files,
    )?;
    let report = Report {
        manifest_digest: manifest.generation_digest.clone(),
        now: Some(month.to_string()),
        lanes: Vec::new(),
        corpus: CorpusReport {
            rows: 1,
            entries: 1,
            skipped: 0,
            notes: 0,
            merges: 0,
        },
        changes: None,
        schedule: Vec::new(),
        outputs: Vec::new(),
        phases: None,
    };
    files.insert(
        "pipeline_report.json".to_string(),
        serde_json::to_vec(&report)?,
    );
    Ok((files, manifest))
}

#[test]
fn crash_boundaries_never_publish_a_mixed_generation() -> TestResult {
    if let Ok(out) = std::env::var("SOL_GENERATION_CHILD") {
        let point = std::env::var("SOL_GENERATION_POINT")?;
        let (files, manifest) = payload("2026-10")?;
        let prefix = manifest
            .generation_digest
            .get(..16)
            .ok_or("missing digest prefix")?;
        publish_with(Path::new(&out), files, &manifest, |position| {
            if format!("{position:?}") == point {
                eprintln!("interrupt at {position:?}; staging {out}/.staging.{}; complete generation {out}/generations/{prefix}", std::process::id());
                std::process::exit(77);
            }
            Ok(())
        })?;
        return Err("fault not reached".into());
    }
    for point in [
        CommitPoint::BeforeDirectory,
        CommitPoint::BeforePointer,
        CommitPoint::AfterPointer,
    ] {
        let dir = TempDir::new()?;
        let (old_files, old_manifest) = payload("2026-09")?;
        publish(dir.path(), old_files, &old_manifest)?;
        let old = manifest::verify_current(dir.path())?;
        let status = std::process::Command::new(std::env::current_exe()?)
            .args(["--exact", "school_address::generation::tests::crash_boundaries_never_publish_a_mixed_generation", "--nocapture"])
            .env("SOL_GENERATION_CHILD", dir.path()).env("SOL_GENERATION_POINT", format!("{point:?}"))
            .status()?;
        check!(eq; status.code(), Some(77));
        let current = manifest::verify_current(dir.path())?;
        let (new_files, new_manifest) = payload("2026-10")?;
        if point == CommitPoint::AfterPointer {
            check!(eq;
                current.directory().file_name().ok_or("missing generation name")?.to_str(),
                new_manifest.generation_digest.get(..16)
            );
            check!(ne;
                current.artifact("manifest.json")?,
                old.artifact("manifest.json")?
            );
            for (name, bytes) in new_files {
                check!(eq; current.artifact(&name)?, bytes);
                check!(ne;
                    current.artifact(&name)?,
                    old.artifact(&name)?
                );
            }
        } else {
            check!(eq; current.directory(), old.directory());
            for name in [
                "school_directory.json",
                "school_directory.csv",
                "baseline.json",
                "update_ledger.json",
                "manifest.json",
                "pipeline_report.json",
            ] {
                check!(eq;
                    current.artifact(name)?,
                    old.artifact(name)?
                );
            }
        }
        let orphan = dir.path().join("generations").join(
            new_manifest
                .generation_digest
                .get(..16)
                .ok_or("missing digest prefix")?,
        );
        check!(eq; orphan.exists(), point != CommitPoint::BeforeDirectory);
        if orphan.exists() {
            manifest::verify_directory(dir.path(), &orphan)?;
        }
        println!(
            "{point:?}: current {} verified; orphan complete={}",
            current.directory().display(),
            orphan.exists()
        );
    }
    Ok(())
}

#[test]
fn manifest_digest_matches_the_adr017_encoder_bytes() -> TestResult {
    let (_, manifest) = payload("2026-09")?;
    let bytes = serde_json::to_vec(&manifest)?;
    check!(eq;
        census_domain::model::serialized_digest(&manifest)?,
        crate::school_address::export::sha256_hex(&bytes)
    );
    Ok(())
}

#[test]
fn consumers_reject_hash_digest_report_and_pointer_mismatches() -> TestResult {
    use crate::school_address::GenerationError;
    for name in [
        "school_directory.csv",
        "manifest.json",
        "pipeline_report.json",
    ] {
        let dir = TempDir::new()?;
        let (files, manifest) = payload("2026-09")?;
        publish(dir.path(), files, &manifest)?;
        let current = manifest::verify_current(dir.path())?;
        let path = current.directory().join(name);
        if name == "school_directory.csv" {
            std::fs::write(&path, b"corrupt")?;
        } else {
            let mut value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
            value[if name == "manifest.json" {
                "generation_digest"
            } else {
                "manifest_digest"
            }] = "forged".into();
            std::fs::write(&path, serde_json::to_vec(&value)?)?;
        }
        let error = match manifest::verify_current(dir.path()) {
            Ok(_) => return Err("tampered generation accepted".into()),
            Err(error) => error,
        };
        println!("{name}: {error}");
        match name {
            "school_directory.csv" => check!(
                matches!(&error, GenerationError::Artifact(name) if name == "school_directory.csv")
            ),
            "manifest.json" => check!(matches!(error, GenerationError::Digest)),
            _ => check!(matches!(error, GenerationError::Report)),
        }
    }
    let dir = TempDir::new()?;
    let (files, manifest) = payload("2026-09")?;
    publish(dir.path(), files, &manifest)?;
    let outside = TempDir::new()?;
    std::fs::remove_file(dir.path().join("current"))?;
    std::os::unix::fs::symlink(outside.path(), dir.path().join("current"))?;
    check!(matches!(
        manifest::verify_current(dir.path()),
        Err(GenerationError::Pointer(_))
    ));
    Ok(())
}
