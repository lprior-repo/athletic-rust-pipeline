use super::{snapshot, Runtime};
use crate::{
    domain::identity::WorkbookDigest,
    model::{SourceRecord, WorkbookStats},
    store::ArtifactStore,
    workbook_ingest,
};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};

const IMPORT_BATCH_ROWS: usize = 512;
const IMPORT_BATCH_BYTES: usize = 8 * 1024 * 1024;
pub const INGESTION_REVISION: &str = "calamine-ooxml-v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRequest {
    pub original: PathBuf,
    pub workbook: WorkbookDigest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceManifest {
    pub original: PathBuf,
    pub frozen: PathBuf,
    pub workbook: WorkbookDigest,
    pub ingestion_revision: String,
    pub stats: WorkbookStats,
}

pub async fn import(runtime: Arc<Runtime>, request: ImportRequest) -> Result<SourceManifest> {
    let store = runtime.store.clone();
    let directory = runtime.config.storage_dir().to_owned();
    runtime
        .blocking(move || {
            let frozen = snapshot::freeze(&request.original, &directory, &request.workbook)?;
            let stats = import_rows(&store, &request.workbook, &frozen)?;
            snapshot::verify(&request.original, &request.workbook)?;
            Ok(SourceManifest {
                original: request.original,
                frozen,
                workbook: request.workbook,
                ingestion_revision: INGESTION_REVISION.to_owned(),
                stats,
            })
        })
        .await
}

fn import_rows(
    store: &ArtifactStore,
    workbook: &WorkbookDigest,
    path: &std::path::Path,
) -> Result<WorkbookStats> {
    let mut batch = ImportBatch {
        store,
        workbook,
        records: Vec::with_capacity(IMPORT_BATCH_ROWS),
        bytes: 0,
    };
    let stats = workbook_ingest::visit_records(path, |record| batch.push(record))?;
    batch.flush()?;
    if stats.sheets.is_empty() || stats.actual_data_rows == 0 {
        bail!("source workbook contains no data rows");
    }
    Ok(stats)
}

struct ImportBatch<'a> {
    store: &'a ArtifactStore,
    workbook: &'a WorkbookDigest,
    records: Vec<SourceRecord>,
    bytes: usize,
}

impl ImportBatch<'_> {
    fn push(&mut self, record: SourceRecord) -> Result<()> {
        let size = record
            .fields
            .iter()
            .try_fold(0_usize, |total, (key, value)| {
                total
                    .checked_add(key.len())
                    .and_then(|value_len| value_len.checked_add(value.len()))
                    .ok_or_else(|| anyhow::anyhow!("source field byte count overflow"))
            })?;
        if !self.records.is_empty()
            && (self.records.len() == IMPORT_BATCH_ROWS
                || self
                    .bytes
                    .checked_add(size)
                    .is_none_or(|bytes| bytes > IMPORT_BATCH_BYTES))
        {
            self.flush()?;
        }
        self.bytes = self
            .bytes
            .checked_add(size)
            .ok_or_else(|| anyhow::anyhow!("source batch size overflow"))?;
        self.records.push(record);
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        if !self.records.is_empty() {
            self.store.put_source_batch(self.workbook, &self.records)?;
            self.records.clear();
            self.bytes = 0;
        }
        Ok(())
    }
}
