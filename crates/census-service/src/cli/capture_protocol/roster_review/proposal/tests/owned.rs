use census_crawl::net::{FetchOptions, Fetcher};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{RetainedCaptureRootSchemaV1, Store};
use clap::Parser;
use sha2::{Digest, Sha256};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

mod binding;
#[path = "../../../../../../tests/common/controlled_public_capture.rs"]
mod controlled_normalized_transport;
mod fixture;
mod profile_publication;
mod source;
mod verify;

const TEAM: &str = "12439";
const ROSTER_URL: &str = "https://mi.milesplit.com/teams/12439-detroit-catholic-central/roster";
const OPERATION: &str = "controlled-normalized-genuine-owned-cli-review";
const RATIONALE: &str = "Genuine retained source bodies with controlled normalized transport; exact representation review only";
const INDEX: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/src/participation_evidence/provider_school/fixtures/mi-team-index.html"
));
const ROSTER: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../census-crawl/src/participation_evidence/provider_school/fixtures/dcc-provider-roster.html"
));

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn invoke(root: &std::path::Path, args: &[&str]) -> TestResult<Vec<u8>> {
    let cli = crate::cli::Cli::try_parse_from(
        std::iter::once("census-service").chain(args.iter().copied()),
    )?;
    let crate::cli::command::Command::CaptureProtocol(args) = cli.command else {
        return Err("capture-protocol fixture command required".into());
    };
    let mut output = Vec::new();
    crate::cli::capture_protocol::run_to(Some(root), &args, &mut output)?;
    Ok(output)
}

#[tokio::test]
async fn genuine_owned_cli_review_previews_applies_and_cold_replays_controlled_normalized_source(
) -> TestResult {
    let fixture = fixture::Fixture::new().await?;
    let before = fixture::checkpoint(&fixture.path)?;
    let derived = verify::derive_with_original_grant(&fixture)?;
    check!(eq; fixture::checkpoint(&fixture.path)?, before);
    let arguments = fixture.arguments();
    let preview = invoke(&fixture.path, &arguments)?;
    check!(eq; preview, derived);
    check!(!preview.is_empty());
    check!(preview.last() != Some(&b'\n'));
    check!(eq; fixture::checkpoint(&fixture.path)?, before);
    let digest = sha256(&preview);
    let mut wrong = arguments.clone();
    let wrong_digest = "0".repeat(64);
    wrong.extend(["--approve-sha256", &wrong_digest]);
    check!(invoke(&fixture.path, &wrong).is_err());
    check!(eq; fixture::checkpoint(&fixture.path)?, before);
    let mut approved = arguments;
    approved.extend(["--approve-sha256", &digest]);
    let applied: serde_json::Value = serde_json::from_slice(&invoke(&fixture.path, &approved)?)?;
    check!(eq; applied["schema"], "roster-review");
    check!(eq; applied["status"], "reviewed_applied");
    check!(eq; applied["packet_reference"], fixture.reference.as_str());
    check!(eq; applied["scoped_rows"], 33);
    check!(eq; applied["approval_digest"], digest.as_str());
    check!(eq; applied["repeated"], false);
    let committed = fixture::checkpoint(&fixture.path)?;
    check!(committed.0 > before.0);
    check!(eq; committed.1, before.1.checked_add(1).ok_or("review receipt count")?);
    let replay: serde_json::Value = serde_json::from_slice(&invoke(
        &fixture.path,
        &[
            "capture-protocol",
            "roster-replay",
            "--operation",
            OPERATION,
            "--approval-sha256",
            &digest,
        ],
    )?)?;
    check!(eq; replay["status"], "replayed");
    check!(eq; replay["repeated"], true);
    check!(eq; replay["receipt_operation"], applied["receipt_operation"]);
    check!(eq; replay["receipt_digest"], applied["receipt_digest"]);
    check!(eq; fixture::checkpoint(&fixture.path)?, committed);
    verify::cold(&fixture)?;
    check!(eq; fixture::checkpoint(&fixture.path)?, committed);
    Ok(())
}
