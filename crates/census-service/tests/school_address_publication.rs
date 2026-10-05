#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_service::school_address::{self, SchoolAddressArgs};
use std::path::Path;
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn std::error::Error>>;

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
        association_directory: Vec::new(),
        out: out.into(),
        baseline: None,
        ledger: None,
        now: None,
        geocode: false,
        validate_postal: false,
    }
}

#[test]
fn blocking_destination_preserves_previous_publication() -> TestResult {
    let dir = TempDir::new()?;
    let blocker = dir.path().join("school_directory.csv");
    std::fs::create_dir(&blocker)?;
    let error = match school_address::run(&args(dir.path())) {
        Err(error) => error,
        Ok(_) => return Err("blocking destination accepted".into()),
    };
    println!("blocking diagnostic: {error:#}");
    check!(format!("{error:#}").contains(&blocker.display().to_string()));
    check!(
        !dir.path().join("school_directory.json").exists(),
        "no early JSON publication"
    );
    check!(
        !dir.path().join("generations").exists(),
        "no staging before preflight"
    );
    Ok(())
}

#[test]
fn unmanifested_external_baseline_is_not_consumed_or_republished() -> TestResult {
    let input = TempDir::new()?;
    let out = TempDir::new()?;
    let path = input.path().join("baseline.json");
    std::fs::write(&path, b"{\"entries\":[]}")?;
    let mut request = args(out.path());
    request.baseline = Some(path.clone());
    request.now = Some("2026-09".to_string());
    let error = match school_address::run(&request) {
        Err(error) => error,
        Ok(_) => return Err("unmanifested baseline accepted".into()),
    };
    check!(
        matches!(error.downcast_ref::<school_address::GenerationError>(), Some(school_address::GenerationError::Destination { path: rejected, .. }) if rejected == &path)
    );
    check!(!out.path().join("generations").exists());
    check!(eq; std::fs::read(&path)?,
    b"{\"entries\":[]}");
    println!("unmanifested baseline diagnostic: {error:#}");
    Ok(())
}

#[test]
fn missing_manifest_artifact_refuses_readback_and_preserves_current() -> TestResult {
    let out = TempDir::new()?;
    school_address::run(&args(out.path()))?;
    let current = std::fs::read_link(out.path().join("current"))?;
    let blocked = out.path().join(&current).join("school_directory.csv");
    std::fs::remove_file(&blocked)?;
    let mut request = args(out.path());
    request.baseline = Some(out.path().join("current/baseline.json"));
    request.now = Some("2026-09".to_string());
    let error = match school_address::run(&request) {
        Err(error) => error,
        Ok(_) => return Err("generation with missing artifact accepted".into()),
    };
    check!(
        matches!(error.downcast_ref::<school_address::GenerationError>(), Some(school_address::GenerationError::Io { path, source }) if path == &blocked && source.kind() == std::io::ErrorKind::NotFound)
    );
    check!(eq; std::fs::read_link(out.path().join("current"))?,
    current);
    println!("missing artifact diagnostic: {error:#}");
    Ok(())
}
