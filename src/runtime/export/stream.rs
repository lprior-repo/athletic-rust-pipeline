use super::{
    details::{pending_fields, write_detail},
    index::count_resolution,
    projection::project_report,
    ExportCoverage,
};
use crate::{
    domain::identity::{EvidenceDigest, WorkbookDigest},
    model::SheetStats,
    store::ArtifactStore,
    workbook_export::{ExportRow, WorkbookExport},
};
use anyhow::{bail, Context, Result};
use std::{collections::BTreeMap, io::Write};

pub(super) fn sources<W: Write>(
    store: &ArtifactStore,
    manifest: &super::SourceManifest,
    reports: &BTreeMap<String, EvidenceDigest>,
    workbook: &mut WorkbookExport,
    detail: &mut W,
    coverage: &mut ExportCoverage,
) -> Result<()> {
    manifest.stats.sheets.iter().try_for_each(|sheet| {
        stream_sheet(
            store,
            &manifest.workbook,
            sheet,
            reports,
            workbook,
            detail,
            coverage,
        )
    })
}

fn stream_sheet<W: Write>(
    store: &ArtifactStore,
    workbook_digest: &WorkbookDigest,
    sheet: &SheetStats,
    reports: &BTreeMap<String, EvidenceDigest>,
    workbook: &mut WorkbookExport,
    detail: &mut W,
    coverage: &mut ExportCoverage,
) -> Result<()> {
    let mut after = 1_u32;
    let mut seen = 0_u64;
    while seen < sheet.actual_data_rows {
        let remaining = sheet
            .actual_data_rows
            .checked_sub(seen)
            .context("source row count underflow")?;
        let limit =
            usize::try_from(remaining.min(256)).context("source page size conversion overflow")?;
        let keys = store.source_key_page(workbook_digest, &sheet.name, after, limit)?;
        validate_page(&keys, &sheet.name, after, limit)?;
        keys.iter().try_for_each(|key| -> Result<()> {
            let source = store
                .source_record(workbook_digest, key)?
                .context("source record missing from index")?;
            let report_digest = reports.get(source.source_key.as_str());
            let projection = report_digest
                .map(|digest| project_report(store, digest))
                .transpose()?;
            let fields = projection.as_ref().map_or_else(
                || pending_fields(&source.source_key),
                |value| value.fields.clone(),
            );
            workbook.write(ExportRow {
                source: source.clone(),
                extra_fields: fields,
            })?;
            write_detail(detail, source, report_digest.cloned(), projection)?;
            record_coverage(coverage, report_digest, store)?;
            Ok(())
        })?;
        after = keys
            .last()
            .map(|key| key.row())
            .context("source key page has no final row")?;
        seen = seen
            .checked_add(
                u64::try_from(keys.len()).context("source page count conversion overflow")?,
            )
            .context("source row count overflow")?;
    }
    if seen != sheet.actual_data_rows {
        bail!("source worksheet cardinality differs from manifest");
    }
    Ok(())
}

fn validate_page(
    keys: &[crate::domain::identity::SourceRowKey],
    sheet: &str,
    after: u32,
    limit: usize,
) -> Result<()> {
    if keys.is_empty() || keys.len() > limit {
        bail!("source key page ended before manifest cardinality");
    }
    keys.iter().try_for_each(|key| {
        if key.sheet() != sheet || key.row() <= after {
            bail!("source key page is not ordered");
        }
        Ok(())
    })
}

fn record_coverage(
    coverage: &mut ExportCoverage,
    report_digest: Option<&EvidenceDigest>,
    store: &ArtifactStore,
) -> Result<()> {
    coverage.source_rows = coverage
        .source_rows
        .checked_add(1)
        .context("source row count overflow")?;
    match report_digest {
        Some(digest) => {
            coverage.completed_rows = coverage
                .completed_rows
                .checked_add(1)
                .context("completed row count overflow")?;
            count_resolution(coverage, digest, store)
        }
        None => {
            coverage.pending_rows = coverage
                .pending_rows
                .checked_add(1)
                .context("pending row count overflow")?;
            Ok(())
        }
    }
}
