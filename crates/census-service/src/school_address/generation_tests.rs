use super::*;
use crate::school_address::manifest::{Inputs, Manifest};
use crate::school_address::{CorpusReport, Report};
use census_domain::school_directory::{
    Baseline, IdentifiedKey, NcesSchoolId, ScheduleLedger, ScheduleSource, SchoolDirectoryEntry,
    SchoolName, SourceLabel,
};
use tempfile::TempDir;

fn payload(month: &str) -> (BTreeMap<String, Vec<u8>>, Manifest) {
    let entries = vec![SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("010000500870").expect("id")),
        SourceLabel::Ccd,
        Some(SchoolName::parse(month).expect("name")),
    )];
    let mut ledger = ScheduleLedger::new();
    ledger.record(ScheduleSource::Ccd, month.parse().expect("month"));
    let mut files = BTreeMap::from([
        (
            "school_directory.json".to_string(),
            super::super::export::json(&entries).expect("entries"),
        ),
        (
            "school_directory.csv".to_string(),
            super::super::export::entries_csv(&entries).expect("csv"),
        ),
        (
            "baseline.json".to_string(),
            super::super::export::json(&Baseline::new(entries)).expect("baseline"),
        ),
        (
            "update_ledger.json".to_string(),
            super::super::export::json(&ledger).expect("ledger"),
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
    )
    .expect("manifest");
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
    };
    files.insert(
        "pipeline_report.json".to_string(),
        serde_json::to_vec(&report).expect("report"),
    );
    (files, manifest)
}

#[test]
fn crash_boundaries_never_publish_a_mixed_generation() {
    if let Ok(out) = std::env::var("SOL_GENERATION_CHILD") {
        let point = std::env::var("SOL_GENERATION_POINT").expect("fault point");
        let (files, manifest) = payload("2026-10");
        publish_with(Path::new(&out), files, &manifest, |position| {
            if format!("{position:?}") == point {
                eprintln!("interrupt at {position:?}; staging {out}/.staging.{}; complete generation {out}/generations/{}", std::process::id(), manifest.generation_digest.get(..16).expect("prefix"));
                std::process::exit(77);
            }
            Ok(())
        }).expect("child publication reaches fault");
        panic!("fault not reached");
    }
    for point in [
        CommitPoint::BeforeDirectory,
        CommitPoint::BeforePointer,
        CommitPoint::AfterPointer,
    ] {
        let dir = TempDir::new().expect("scratch");
        let (old_files, old_manifest) = payload("2026-09");
        publish(dir.path(), old_files, &old_manifest).expect("old generation");
        let old = manifest::verify_current(dir.path()).expect("old verified");
        let status = std::process::Command::new(std::env::current_exe().expect("test executable"))
            .args(["--exact", "school_address::generation::tests::crash_boundaries_never_publish_a_mixed_generation", "--nocapture"])
            .env("SOL_GENERATION_CHILD", dir.path()).env("SOL_GENERATION_POINT", format!("{point:?}"))
            .status().expect("crash child");
        assert_eq!(status.code(), Some(77));
        let current = manifest::verify_current(dir.path())
            .expect("current is a complete verified generation");
        let (new_files, new_manifest) = payload("2026-10");
        if point == CommitPoint::AfterPointer {
            assert_eq!(
                current.directory().file_name().expect("name").to_str(),
                new_manifest.generation_digest.get(..16)
            );
            assert_ne!(
                current.artifact("manifest.json").expect("manifest"),
                old.artifact("manifest.json").expect("old manifest")
            );
            for (name, bytes) in new_files {
                assert_eq!(current.artifact(&name).expect("new artifact"), bytes);
                assert_ne!(
                    current.artifact(&name).expect("new artifact"),
                    old.artifact(&name).expect("old artifact")
                );
            }
        } else {
            assert_eq!(current.directory(), old.directory());
            for name in [
                "school_directory.json",
                "school_directory.csv",
                "baseline.json",
                "update_ledger.json",
                "manifest.json",
                "pipeline_report.json",
            ] {
                assert_eq!(
                    current.artifact(name).expect("current artifact"),
                    old.artifact(name).expect("old artifact")
                );
            }
        }
        let orphan = dir
            .path()
            .join("generations")
            .join(new_manifest.generation_digest.get(..16).expect("prefix"));
        assert_eq!(orphan.exists(), point != CommitPoint::BeforeDirectory);
        if orphan.exists() {
            manifest::verify_directory(dir.path(), &orphan).expect("orphan is complete");
        }
        println!(
            "{point:?}: current {} verified; orphan complete={}",
            current.directory().display(),
            orphan.exists()
        );
    }
}

#[test]
fn manifest_digest_matches_the_adr017_encoder_bytes() {
    let (_, manifest) = payload("2026-09");
    let bytes = serde_json::to_vec(&manifest).expect("canonical manifest");
    assert_eq!(
        census_domain::model::serialized_digest(&manifest).expect("domain encoder"),
        crate::school_address::export::sha256_hex(&bytes)
    );
}

#[test]
fn consumers_reject_hash_digest_report_and_pointer_mismatches() {
    use crate::school_address::GenerationError;
    for name in [
        "school_directory.csv",
        "manifest.json",
        "pipeline_report.json",
    ] {
        let dir = TempDir::new().expect("scratch");
        let (files, manifest) = payload("2026-09");
        publish(dir.path(), files, &manifest).expect("generation");
        let current = manifest::verify_current(dir.path()).expect("verified");
        let path = current.directory().join(name);
        if name == "school_directory.csv" {
            std::fs::write(&path, b"corrupt").expect("tamper artifact");
        } else {
            let mut value: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).expect("bytes")).expect("json");
            value[if name == "manifest.json" {
                "generation_digest"
            } else {
                "manifest_digest"
            }] = "forged".into();
            std::fs::write(&path, serde_json::to_vec(&value).expect("json"))
                .expect("tamper digest");
        }
        let error = match manifest::verify_current(dir.path()) {
            Ok(_) => panic!("tampered generation accepted"),
            Err(error) => error,
        };
        println!("{name}: {error}");
        match name {
            "school_directory.csv" => assert!(
                matches!(&error, GenerationError::Artifact(name) if name == "school_directory.csv")
            ),
            "manifest.json" => assert!(matches!(error, GenerationError::Digest)),
            _ => assert!(matches!(error, GenerationError::Report)),
        }
    }
    let dir = TempDir::new().expect("scratch");
    let (files, manifest) = payload("2026-09");
    publish(dir.path(), files, &manifest).expect("generation");
    let outside = TempDir::new().expect("outside");
    std::fs::remove_file(dir.path().join("current")).expect("remove pointer");
    std::os::unix::fs::symlink(outside.path(), dir.path().join("current"))
        .expect("outside pointer");
    assert!(matches!(
        manifest::verify_current(dir.path()),
        Err(GenerationError::Pointer(_))
    ));
}
