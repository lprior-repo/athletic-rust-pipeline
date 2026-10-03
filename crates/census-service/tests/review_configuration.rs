use anyhow::{ensure, Result};
use std::process::Command;

#[test]
fn invalid_review_configuration_cannot_create_a_store() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let cases: &[(&str, &[&str])] = &[
        (
            "remote",
            &[
                "--endpoint",
                "http://192.0.2.1:11000",
                "--endpoint",
                "http://127.0.0.1:11001",
            ],
        ),
        (
            "duplicate",
            &[
                "--endpoint",
                "http://127.0.0.1:11000",
                "--endpoint",
                "http://127.0.0.1:11000",
            ],
        ),
        ("empty-model", &["--model", " "]),
        ("unknown-family", &["--family", "not-a-review-family"]),
    ];
    for (name, options) in cases {
        let root = directory.path().join(name);
        let result = Command::new(env!("CARGO_BIN_EXE_census-service"))
            .arg("--store")
            .arg(&root)
            .arg("review")
            .args(*options)
            .output()?;
        ensure!(
            !result.status.success(),
            "{name} invalid configuration succeeded"
        );
        ensure!(
            !root.exists(),
            "{name} invalid configuration created a store: {}",
            root.display()
        );
    }
    Ok(())
}
