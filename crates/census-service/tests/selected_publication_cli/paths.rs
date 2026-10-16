use super::{fixture::Fixture, run, TestResult};
use std::process::Command;

#[tokio::test]
async fn selected_publication_refuses_unsafe_paths_without_overwriting() -> TestResult {
    let fixture = Fixture::create().await?;
    let directory = fixture.directory.path();
    let file = directory.join("keep.txt");
    std::fs::write(&file, b"retained user bytes")?;
    let existing = directory.join("existing");
    std::fs::create_dir(&existing)?;
    std::fs::write(existing.join("keep"), b"retained directory bytes")?;
    for path in [
        &file,
        &existing,
        &fixture.root.join("new-output"),
        &directory.join("absent-parent/output"),
    ] {
        let result = run(&fixture.root, &fixture.selection, path)?;
        check!(!result.status.success());
    }
    check!(eq; std::fs::read(&file)?, b"retained user bytes");
    check!(eq; std::fs::read(existing.join("keep"))?, b"retained directory bytes");
    check!(!fixture.root.join("new-output").exists());
    check!(!directory.join("absent-parent").exists());
    let absent = directory.join("absent-store");
    let out = directory.join("absent-store-output");
    check!(!run(&absent, &fixture.selection, &out)?.status.success());
    check!(!absent.exists());
    check!(!out.exists());
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn selected_publication_refuses_output_symlinks_and_store_parent_aliases() -> TestResult {
    let fixture = Fixture::create().await?;
    let directory = fixture.directory.path();
    let dangling = directory.join("dangling");
    std::os::unix::fs::symlink("missing-target", &dangling)?;
    check!(!run(&fixture.root, &fixture.selection, &dangling)?
        .status
        .success());
    check!(std::fs::symlink_metadata(&dangling)?
        .file_type()
        .is_symlink());
    let alias = directory.join("store-alias");
    std::os::unix::fs::symlink(&fixture.root, &alias)?;
    check!(
        !run(&fixture.root, &fixture.selection, &alias.join("output"))?
            .status
            .success()
    );
    check!(!fixture.root.join("output").exists());
    Ok(())
}

#[test]
fn selected_publication_rejects_independent_profile_overrides() -> TestResult {
    let directory = tempfile::tempdir()?;
    for (flag, value) in [
        ("--grad-year", "2028"),
        ("--school-year", "2025"),
        ("--limit", "1"),
        ("--scope", "core"),
        ("--jurisdictions", "WI"),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_census-service"))
            .current_dir(directory.path())
            .args([
                "selected-publication",
                "--selection",
                "missing",
                "--out",
                "output",
                flag,
                value,
            ])
            .output()?;
        check!(!result.status.success());
        check!(String::from_utf8_lossy(&result.stderr).contains("unexpected argument"));
        check!(!directory.path().join("output").exists());
    }
    Ok(())
}
