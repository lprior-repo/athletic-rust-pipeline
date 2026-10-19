use census_store::{Receipt, Store};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const REVIEW: [&str; 8] = [
    "capture-protocol",
    "roster-enrollment",
    "--operation",
    "cli-roster-capability",
    "--reviewed-at",
    "2026-10-01T00:00:00Z",
    "--rationale",
    "Explicit capability only; no membership accepted",
];

fn invoke(root: &Path, args: &[&str]) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_census-service"))
        .arg("--store")
        .arg(root)
        .args(args)
        .output()?)
}

fn prepared() -> TestResult<(tempfile::TempDir, PathBuf, String)> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("current");
    for command in ["init", "enable-admission"] {
        let output = invoke(&root, &["capture-protocol", command])?;
        check!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let preview = invoke(&root, &REVIEW)?;
    check!(
        preview.status.success(),
        "{}",
        String::from_utf8_lossy(&preview.stderr)
    );
    check!(eq; Store::format(&root)?.lifecycle_revision, Some(7));
    let digest = format!("{:x}", Sha256::digest(&preview.stdout));
    Ok((directory, root, digest))
}

fn checkpoint(root: &Path) -> TestResult<(u64, u64, Option<u32>)> {
    let (store, _writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
    Ok((
        store.snapshot().sequence(),
        store.receipt_count()?,
        store.store_format()?.lifecycle_revision,
    ))
}

#[test]
fn roster_preview_and_wrong_approval_do_not_enroll_or_replace_state() -> TestResult {
    let (_directory, root, _digest) = prepared()?;
    let before = checkpoint(&root)?;
    let wrong = "0".repeat(64);
    let mut args = REVIEW.to_vec();
    args.extend(["--approve-sha256", wrong.as_str()]);
    let refused = invoke(&root, &args)?;
    check!(!refused.status.success());
    check!(refused.stdout.is_empty());
    check!(eq; checkpoint(&root)?, before);
    Ok(())
}

#[test]
fn exact_opaque_preview_enrolls_and_replays_one_original_receipt() -> TestResult {
    let (_directory, root, digest) = prepared()?;
    let capture = Store::capture_qualification_control_existing(&root)?.show()?;
    let mut args = REVIEW.to_vec();
    args.extend(["--approve-sha256", digest.as_str()]);
    let first = invoke(&root, &args)?;
    check!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let receipt: Receipt = serde_json::from_slice(&first.stdout)?;
    check!(eq; receipt.appended, 0);
    let before = checkpoint(&root)?;
    check!(eq; before.2, Some(8));
    let repeated = invoke(&root, &args)?;
    check!(
        repeated.status.success(),
        "{}",
        String::from_utf8_lossy(&repeated.stderr)
    );
    check!(eq; serde_json::from_slice::<Receipt>(&repeated.stdout)?, receipt);
    check!(eq; checkpoint(&root)?, before);
    check!(eq; Store::capture_qualification_control_existing(&root)?.show()?, capture);
    Ok(())
}
