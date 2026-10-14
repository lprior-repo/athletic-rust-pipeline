use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn post_create_metadata_failure_poison_refuses_unresolved_namespace_ownership() -> TestResult {
    let root = tempfile::tempdir()?;
    let path = root.path().join(".results-source-spill");
    create_private(&path)?;
    std::fs::remove_dir(&path)?;
    let admission = Arc::new(SpillAdmission::default());
    let lease = admission.claim()?;
    let error = Directory::finish_created(path, Vec::new(), Arc::clone(&admission))
        .err()
        .ok_or("missing created namespace accepted")?;
    check!(error.to_string().contains("ownership unresolved"));
    drop(lease);
    check!(admission.claim().is_err());
    Ok(())
}
