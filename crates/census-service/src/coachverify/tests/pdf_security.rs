use super::TestResult;
use std::os::unix::fs::symlink;
use std::time::Duration;

const CHILD_MODE: &str = "CENSUS_COACH_SCRATCH_SECURITY_CHILD";
const TEST_NAME: &str =
    "coachverify::tests::pdf_security::conversion_preserves_preexisting_shared_scratch_links";

#[test]
fn conversion_preserves_preexisting_shared_scratch_links() -> TestResult {
    if std::env::var_os(CHILD_MODE).is_some() {
        return poisoned_shared_names();
    }
    let scratch = tempfile::tempdir()?;
    let output = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", TEST_NAME, "--nocapture"])
        .env(CHILD_MODE, "1")
        .env("TMPDIR", scratch.path())
        .output()?;
    check!(
        output.status.success(),
        "isolated scratch ownership failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn poisoned_shared_names() -> TestResult {
    let scratch = std::env::temp_dir();
    let victim = scratch.join("retained-victim");
    std::fs::write(&victim, b"untouched victim")?;
    let tag = format!("coachverify-{}-0", std::process::id());
    let raw = scratch.join(format!("{tag}.pdf"));
    let text = scratch.join(format!("{tag}.txt"));
    symlink(&victim, &raw)?;
    symlink(&victim, &text)?;
    let converted = super::super::fetch::inflate_pdf_under(
        "/bin/false",
        b"%PDF-1.4 adversarial scratch content",
        Duration::from_secs(2),
        4096,
    );
    check!(converted.is_none(), "failing converter must remain refused");
    let retained = std::fs::read(&victim)?;
    check!(eq; retained.as_slice(), b"untouched victim".as_slice(),
        "downloaded PDF bytes must not overwrite a preexisting scratch-link target");
    check!(std::fs::symlink_metadata(&raw)?.file_type().is_symlink());
    check!(std::fs::symlink_metadata(&text)?.file_type().is_symlink());
    Ok(())
}
