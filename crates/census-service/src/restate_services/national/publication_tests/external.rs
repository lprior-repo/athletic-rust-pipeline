use super::{codec, fixtures, journal::Journal, transport, TestResult};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Duration;

mod config;
mod tests;

#[tokio::test]
#[ignore = "requires explicit old/current isolated endpoints and fresh evidence directory; not native durability"]
async fn historical_publication_prefix_replays_between_explicit_endpoints() -> TestResult {
    let settings = config::Settings::read(|key| std::env::var(key).ok())?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .http2_prior_knowledge()
        .connect_timeout(Duration::from_secs(1))
        .timeout(Duration::from_secs(5))
        .build()?;
    std::fs::create_dir(&settings.evidence)?;
    transport::write_new(
        &settings.evidence.join("configuration.json"),
        &serde_json::to_vec_pretty(&serde_json::json!({
            "schema": 1, "evidence_kind": "sdk_replay_with_synthetic_child_completions",
            "old_endpoint": settings.old_url, "current_endpoint": settings.current_url,
            "old_artifact": settings.old, "current_artifact": settings.current,
            "process_identity_authenticated": false, "native_restate_durability": false,
            "isolation": "operator-attested; separate initially empty scratch stores; provider/model egress disabled"
        }))?,
    )?;
    let result = matrix(
        &client,
        &settings.old_url,
        &settings.current_url,
        &settings.evidence,
    )
    .await;
    let outcome = match &result {
        Ok(()) => {
            serde_json::json!({"protocol_replay": "passed", "native_restate_durability": "not_run"})
        }
        Err(error) => {
            serde_json::json!({"protocol_replay": "failed", "error": error.to_string(), "native_restate_durability": "not_run"})
        }
    };
    transport::write_new(
        &settings.evidence.join("outcome.json"),
        &serde_json::to_vec_pretty(&outcome)?,
    )?;
    result
}

pub(super) async fn local_control(client: &reqwest::Client, url: &str) -> TestResult {
    let root = tempfile::tempdir()?;
    matrix(client, url, url, root.path()).await
}

async fn matrix(client: &reqwest::Client, old: &str, current: &str, root: &Path) -> TestResult {
    for stage in ["Report", "Bests", "Workbook"] {
        let path = root.join(stage);
        std::fs::create_dir(&path)?;
        tokio::time::timeout(
            Duration::from_secs(30),
            scenario(client, old, current, &path, stage),
        )
        .await??;
    }
    Ok(())
}

fn fixture(stage: &str) -> TestResult<Journal> {
    let mut request = fixtures::request()?;
    request.jurisdictions = vec![census_domain::UsJurisdiction::Wisconsin];
    let mut journal = Journal::new(&request)?;
    journal.invocation = format!("invocation-historical-publication-{stage}");
    journal.hold_stage = Some(stage.to_owned());
    Ok(journal)
}

async fn scenario(
    client: &reqwest::Client,
    old: &str,
    current: &str,
    root: &Path,
    stage: &str,
) -> TestResult {
    let mut journal = fixture(stage)?;
    journal.transcript = Some(transport::Transcript::new(root.to_path_buf()));
    journal.drive(client, old).await?;
    check!(eq; journal.held.as_ref().ok_or("old endpoint did not hold publication")?.text(1)?, stage);
    check!(
        journal.output.is_none() && !journal.ended,
        "old endpoint completed before held publication"
    );
    let prefix = Prefix::capture(&journal)?;
    transport::write_new(&root.join("held-prefix.bin"), &prefix.request)?;
    transport::write_new(
        &root.join("held-prefix.json"),
        &serde_json::to_vec_pretty(&prefix.summary(stage, &journal)?)?,
    )?;
    journal
        .transcript
        .as_mut()
        .ok_or("transcript absent")?
        .candidate();
    let response = journal.step(client, current).await?;
    prefix.check_suspended(&journal, &response)?;
    journal.release()?;
    journal.drive(client, current).await?;
    prefix.check_retained(&journal)?;
    check!(journal.ended, "candidate omitted successful end frame");
    super::assert_published(&journal)?;
    check_suffix(&prefix, &journal, stage)?;
    check_payloads(&journal)?;
    let calls = journal.calls.clone();
    let output = journal.output.clone();
    let request = journal.request_body()?;
    let replay = journal.step(client, current).await?;
    check!(
        replay.iter().any(|frame| frame.kind == 0x0401)
            && replay.iter().any(|frame| frame.kind == 0x0003),
        "completed replay did not return output and end"
    );
    check!(eq; journal.calls, calls);
    check!(eq; journal.output, output);
    check!(eq; journal.request_body()?, request);
    prefix.check_retained(&journal)?;
    transport::write_new(
        &root.join("final-output.json"),
        &serde_json::to_vec_pretty(&journal.output)?,
    )?;
    Ok(())
}

fn check_suffix(prefix: &Prefix, journal: &Journal, stage: &str) -> TestResult {
    let expected: &[&str] = match stage {
        "Report" => &["Bests", "Workbook", "Census"],
        "Bests" => &["Workbook", "Census"],
        "Workbook" => &["Census"],
        _ => return Err("unsupported held stage".into()),
    };
    let suffix = journal
        .calls
        .get(prefix.calls.len()..)
        .ok_or("call prefix shortened")?
        .iter()
        .map(|call| call.text(1))
        .collect::<TestResult<Vec<_>>>()?;
    check!(eq; suffix, expected);
    Ok(())
}

fn check_payloads(journal: &Journal) -> TestResult {
    for (stage, expected) in [
        ("Consolidate", serde_json::json!({"tables": []})),
        (
            "Report",
            serde_json::json!({"scope": "all_sources", "jurisdictions": ["WI"]}),
        ),
        (
            "Bests",
            serde_json::json!({"scope": "all_sources", "grad_year": 2027, "limit": null, "jurisdictions": ["WI"]}),
        ),
        (
            "Workbook",
            serde_json::json!({"scope": "all_sources", "grad_year": 2027, "limit": null, "out": null, "school_year": 2026, "jurisdictions": ["WI"]}),
        ),
    ] {
        let call = journal
            .calls
            .iter()
            .find(|call| call.text(1).is_ok_and(|service| service == stage))
            .ok_or("publication call missing")?;
        check!(eq; call.text(2)?, "run");
        check!(eq; serde_json::from_slice::<serde_json::Value>(call.bytes(3)?)?, expected);
    }
    Ok(())
}

struct Prefix {
    request: Vec<u8>,
    entries: Vec<codec::Frame>,
    calls: Vec<codec::Frame>,
    identity: String,
    invocation: String,
}

impl Prefix {
    fn capture(journal: &Journal) -> TestResult<Self> {
        Ok(Self {
            request: journal.request_body()?,
            entries: journal.entries.clone(),
            calls: journal.calls.clone(),
            identity: journal.identity.clone(),
            invocation: journal.invocation.clone(),
        })
    }

    fn check_prefix(&self, journal: &Journal) -> TestResult {
        check!(eq; journal.request_body()?, self.request);
        check!(eq; journal.calls, self.calls);
        self.check_retained(journal)
    }

    fn check_suspended(&self, journal: &Journal, response: &[codec::Frame]) -> TestResult {
        check!(eq; transport::terminal(response)?, 0x0001);
        let held = journal.held.as_ref().ok_or("held call absent")?;
        let result = u32::try_from(held.number(11)?)?;
        let mut waiting = codec::blob(1, &codec::encode_varint(u64::from(result)))?;
        waiting.extend(codec::blob(2, &[1])?);
        check!(eq; response.last().ok_or("suspension absent")?.payload, waiting);
        check!(
            journal.output.is_none() && !journal.ended,
            "candidate completed before held completion"
        );
        self.check_prefix(journal)
    }

    fn check_retained(&self, journal: &Journal) -> TestResult {
        check!(
            journal.entries.starts_with(&self.entries),
            "historical command entries changed"
        );
        check!(
            journal.calls.starts_with(&self.calls),
            "historical child calls changed"
        );
        check!(eq; journal.identity, self.identity);
        check!(eq; journal.invocation, self.invocation);
        Ok(())
    }

    fn summary(&self, stage: &str, journal: &Journal) -> TestResult<impl Serialize> {
        let original = serde_json::to_value(&journal.original)?;
        check!(
            original.get("publication_contract").is_none(),
            "legacy input acquired a managed contract"
        );
        Ok(serde_json::json!({
            "stage": stage, "sha256": format!("{:x}", Sha256::digest(&self.request)),
            "bytes": self.request.len(), "entries": self.entries.len(), "calls": self.calls.len(),
            "identity": self.identity, "invocation": self.invocation, "input": original,
            "synthetic_child_completions": true, "native_restate_journal": false
        }))
    }
}
