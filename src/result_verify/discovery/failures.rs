use crate::{
    runtime::{
        acquisition::QueryEvidence,
        protocol::{FailureCode, SourceResource},
    },
    search::parse_page,
    store::ArtifactStore,
};
use anyhow::{bail, Context, Result};
use url::Url;

pub(super) fn verify(
    evidence: &QueryEvidence,
    next_start: Option<u32>,
    reconciliation_start: Option<u32>,
    origin: &Url,
    store: &ArtifactStore,
) -> Result<()> {
    if evidence.failures.len() > 1 {
        bail!("query retains failures after its first terminal failure");
    }
    let Some(failure) = evidence.failures.first() else {
        if reconciliation_start.is_some() {
            bail!("query reconciliation failure has no retained failure");
        }
        return Ok(());
    };
    let start = reconciliation_start
        .or(next_start)
        .context("query retains a failure after pagination completed")?;
    failure.evidence.iter().try_for_each(|receipt| {
        super::super::source_receipts::verify_receipt(receipt, origin, store)
    })?;
    if failure.code != FailureCode::MalformedResponse {
        if reconciliation_start.is_some() {
            bail!("query reconciliation failure has an incompatible failure code");
        }
        // Global admission failures may legitimately retain another request's
        // denial receipt; infrastructure failures need not reproduce on replay.
        return Ok(());
    }
    let (receipt, previous) = failure
        .evidence
        .split_last()
        .context("malformed query failure has no captured response")?;
    if failure.http_status != Some(receipt.http_status) {
        bail!("malformed query failure status differs from its captured response");
    }
    let resource = SourceResource::Search {
        query: evidence.query.text().to_owned(),
        sport: evidence.query.sport(),
        start,
    };
    super::super::source_receipts::verify_operation(
        &resource,
        &failure.retries,
        receipt,
        previous,
        origin,
        store,
    )
    .context("verifying malformed query failure operation")?;
    if reconciliation_start.is_some() {
        let page = evidence
            .pages
            .last()
            .context("query reconciliation failure has no final page")?;
        if page.response != *receipt
            || page.previous_responses != previous
            || page.retries != failure.retries
        {
            bail!("query reconciliation failure differs from its final page operation");
        }
    } else {
        let raw = store.get_bytes(&receipt.digest)?;
        if parse_page(&evidence.query, start, receipt.digest.clone(), &raw).is_ok() {
            bail!("malformed query failure has a parseable captured response");
        }
    }
    Ok(())
}
