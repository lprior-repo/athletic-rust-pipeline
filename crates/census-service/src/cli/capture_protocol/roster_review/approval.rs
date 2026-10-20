use anyhow::{ensure, Context, Result};
use census_crawl::participation_evidence::roster_source_issuance::{
    assess_existing_roster_binding, assess_retained_roster_source_v3,
    manifest::compiled_roster_parser_manifest,
    policies::compiled_roster_interpretation_contracts_v1,
};
use census_store::{PreparedRosterSourceReviewV1, Store};
use std::io::Write;

pub(super) fn verify(prepared: &mut PreparedRosterSourceReviewV1<'_>) -> Result<usize> {
    let final_claim = prepared
        .proposal_claim()
        .context("reading the prepared final claim")?;
    let (evidence, grant) = prepared
        .source_parts()
        .context("opening the prepared source parts")?;
    let parser = compiled_roster_parser_manifest().context("compiling the parser manifest")?;
    let contracts = compiled_roster_interpretation_contracts_v1();
    let source = assess_retained_roster_source_v3(
        evidence,
        parser.canonical_bytes(),
        &contracts,
        &final_claim,
        grant,
    )
    .context("assessing the prepared roster source")?;
    let _binding =
        assess_existing_roster_binding(&source, grant).context("assessing the roster binding")?;
    source
        .scoped_rows(grant)
        .context("reading the scoped row count")
}

pub(super) fn conclude<W: Write>(
    store: &Store,
    prepared: PreparedRosterSourceReviewV1<'_>,
    approve_sha256: Option<&str>,
    packet_reference: &str,
    requested_operation: &str,
    scoped: usize,
    output: &mut W,
) -> Result<()> {
    let Some(given_digest) = approve_sha256 else {
        output
            .write_all(prepared.approval_bytes())
            .context("previewing the opaque approval bytes")?;
        return Ok(());
    };
    ensure!(
        given_digest == prepared.approval_digest(),
        "roster review approval differs from the inspected state, proposal and review"
    );
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let applied = store
        .apply_roster_source_review_v1(prepared, &approval, &digest)
        .context("applying the roster source review")?;
    let reply = serde_json::json!({
        "schema": "roster-review",
        "status": "reviewed_applied",
        "packet_reference": packet_reference,
        "requested_operation": requested_operation,
        "scoped_rows": scoped,
        "approval_digest": digest,
        "appended": applied.appended(),
        "repeated": applied.repeated(),
        "receipt_operation": applied.receipt().operation,
        "receipt_digest": applied.receipt().digest,
        "qualification_scope": "explicit local-operator review of the exact retained representation; admits only store-verified rows, mints no identity, confers no affiliation or national acceptance",
    });
    serde_json::to_writer_pretty(&mut *output, &reply)?;
    writeln!(output)?;
    Ok(())
}
