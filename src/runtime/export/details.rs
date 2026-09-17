use super::projection::Projection;
use crate::{domain::identity::EvidenceDigest, model::SourceRecord};
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::io::Write;

#[derive(Debug, Serialize)]
pub(super) struct DetailRow {
    source: SourceRecord,
    discovery: Option<Value>,
    identity_artifacts: Vec<Value>,
    report_digest: Option<EvidenceDigest>,
    report: Option<super::RowReport>,
    assessment: Option<Value>,
    profile_artifacts: Vec<Value>,
    performance_evidence: Vec<Value>,
}
pub(super) fn write_detail<W: Write>(
    detail: &mut W,
    source: SourceRecord,
    digest: Option<EvidenceDigest>,
    projection: Option<Projection>,
) -> Result<()> {
    let (
        discovery,
        identity_artifacts,
        report,
        assessment,
        profile_artifacts,
        performance_evidence,
    ) = match projection {
        None => (None, Vec::new(), None, None, Vec::new(), Vec::new()),
        Some(value) => {
            let performance_evidence = value
                .profile_artifacts
                .iter()
                .flat_map(super::projection::raw_performance_values)
                .collect();
            (
                value.discovery,
                value.identity_artifacts,
                Some(value.report),
                value.assessment,
                value.profile_artifacts,
                performance_evidence,
            )
        }
    };
    let row = DetailRow {
        source,
        discovery,
        identity_artifacts,
        report_digest: digest,
        report,
        assessment,
        profile_artifacts,
        performance_evidence,
    };
    serde_json::to_writer(&mut *detail, &row).context("writing detail row")?;
    detail.write_all(b"\n").context("terminating detail row")
}

pub(super) fn pending_fields(source_key: &str) -> std::collections::BTreeMap<String, String> {
    super::EXPORT_HEADERS
        .iter()
        .map(|header| {
            let value = match *header {
                "native.source_key" => source_key.to_owned(),
                "native.terminal_status" => "PENDING".to_owned(),
                "native.eligibility_basis" => "source_workbook_membership".to_owned(),
                _ => String::new(),
            };
            ((*header).to_owned(), value)
        })
        .collect()
}

pub(super) fn status(resolution: &super::RowResolution) -> &'static str {
    match resolution {
        super::RowResolution::Accepted { .. } => "ACCEPTED",
        super::RowResolution::CompleteSearchNoMatch => "COMPLETE_SEARCH_NO_MATCH",
        super::RowResolution::ReviewRequired => "REVIEW_REQUIRED",
    }
}
