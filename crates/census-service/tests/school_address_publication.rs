use census_service::school_address::{self, SchoolAddressArgs};
use std::path::Path;
use tempfile::TempDir;

fn args(out: &Path) -> SchoolAddressArgs {
    SchoolAddressArgs {
        ccd: Some(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../census-crawl/tests/fixtures/nces/ccd_sch_029_2526_head.csv"
            )
            .into(),
        ),
        pss: None,
        state_ed_index: Vec::new(),
        state_ed_profile: Vec::new(),
        state_ed_tabular: Vec::new(),
        associations: Vec::new(),
        out: out.into(),
        baseline: None,
        ledger: None,
        now: None,
        geocode: false,
        validate_postal: false,
    }
}

#[test]
fn blocking_destination_preserves_previous_publication() {
    let dir = TempDir::new().expect("scratch directory");
    let blocker = dir.path().join("school_directory.csv");
    std::fs::create_dir(&blocker).expect("blocking directory");
    let error = school_address::run(&args(dir.path())).expect_err("refuse blocker");
    println!("blocking diagnostic: {error:#}");
    assert!(format!("{error:#}").contains(&blocker.display().to_string()));
    assert!(
        !dir.path().join("school_directory.json").exists(),
        "no early JSON publication"
    );
    assert!(
        !dir.path().join("generations").exists(),
        "no staging before preflight"
    );
}

#[test]
fn unmanifested_external_baseline_is_not_consumed_or_republished() {
    let input = TempDir::new().expect("legacy input");
    let out = TempDir::new().expect("fresh output");
    let path = input.path().join("baseline.json");
    std::fs::write(&path, b"{\"entries\":[]}").expect("legacy baseline");
    let mut request = args(out.path());
    request.baseline = Some(path.clone());
    request.now = Some("2026-09".to_string());
    let error = school_address::run(&request).expect_err("unmanifested baseline");
    assert!(
        matches!(error.downcast_ref::<school_address::GenerationError>(), Some(school_address::GenerationError::Destination { path: rejected, .. }) if rejected == &path)
    );
    assert!(!out.path().join("generations").exists());
    assert_eq!(
        std::fs::read(&path).expect("legacy bytes unchanged"),
        b"{\"entries\":[]}"
    );
    println!("unmanifested baseline diagnostic: {error:#}");
}

#[test]
fn missing_manifest_artifact_refuses_readback_and_preserves_current() {
    let out = TempDir::new().expect("generation");
    school_address::run(&args(out.path())).expect("publish");
    let current = std::fs::read_link(out.path().join("current")).expect("pointer");
    let blocked = out.path().join(&current).join("school_directory.csv");
    std::fs::remove_file(&blocked).expect("simulate artifact loss");
    let mut request = args(out.path());
    request.baseline = Some(out.path().join("current/baseline.json"));
    request.now = Some("2026-09".to_string());
    let error = school_address::run(&request).expect_err("missing artifact");
    assert!(
        matches!(error.downcast_ref::<school_address::GenerationError>(), Some(school_address::GenerationError::Io { path, source }) if path == &blocked && source.kind() == std::io::ErrorKind::NotFound)
    );
    assert_eq!(
        std::fs::read_link(out.path().join("current")).expect("old pointer"),
        current
    );
    println!("missing artifact diagnostic: {error:#}");
}
