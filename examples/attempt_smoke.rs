use anyhow::{ensure, Context, Result};
use athletic_rust_pipeline::{
    runtime::{acquisition::QueryEvidence, protocol::RetryEvidence, row_protocol::RowReport},
    store::ArtifactStore,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

#[derive(Deserialize)]
struct Detail {
    report: Option<RowReport>,
}

fn main() -> Result<()> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    ensure!(
        arguments.len() == 2,
        "expected store and synthetic detail paths"
    );
    let store_path = arguments.first().context("missing store path")?;
    let detail_path = arguments.get(1).context("missing detail path")?;
    let store = ArtifactStore::open(Path::new(store_path))?;
    let mut detail = String::new();
    File::open(detail_path)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_string(&mut detail)?;
    ensure!(
        detail.len() <= 16 * 1024 * 1024,
        "synthetic detail too large"
    );
    let mut output = std::io::stdout().lock();
    let failures = detail.lines().try_fold(0_usize, |count, line| {
        let row: Detail = serde_json::from_str(line)?;
        let found = inspect_row(&store, row, &mut output)?;
        count.checked_add(found).context("failure count overflow")
    })?;
    ensure!(
        failures == 1,
        "expected exactly one exhausted synthetic source operation"
    );
    Ok(())
}

fn inspect_row(store: &ArtifactStore, row: Detail, output: &mut impl Write) -> Result<usize> {
    let Some(report) = row.report else {
        return Ok(0);
    };
    report
        .query_evidence
        .iter()
        .try_fold(0_usize, |count, digest| {
            let bytes = store.get_bytes(digest)?;
            let query: QueryEvidence = serde_json::from_slice(&bytes)?;
            query.failures.iter().try_fold(count, |count, failure| {
                verify_attempts(store, &failure.retries, output)?;
                count.checked_add(1).context("failure count overflow")
            })
        })
}

fn verify_attempts(
    store: &ArtifactStore,
    retries: &RetryEvidence,
    output: &mut impl Write,
) -> Result<()> {
    let RetryEvidence::SdkControlled {
        operation,
        maximum_retries,
        observed_attempts,
        attempts,
    } = retries
    else {
        anyhow::bail!("failure lacks SDK-controlled attempt evidence");
    };
    ensure!(
        *observed_attempts == 4 && attempts.len() == 4,
        "expected initial attempt plus three retries"
    );
    ensure!(
        serde_json::to_value(maximum_retries)? == json!(3),
        "unexpected retry budget"
    );
    attempts.iter().try_for_each(|digest| {
        store.read_attempt::<Value>(operation, digest)?;
        Ok::<(), anyhow::Error>(())
    })?;
    serde_json::to_writer(
        &mut *output,
        &json!({
            "operation": operation,
            "maximum_retries": maximum_retries,
            "observed_attempts": observed_attempts,
            "verified_attempt_artifacts": attempts.len(),
        }),
    )?;
    output.write_all(b"\n")?;
    Ok(())
}
