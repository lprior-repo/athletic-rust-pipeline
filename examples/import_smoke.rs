use anyhow::{ensure, Context, Result};
use athletic_rust_pipeline::{
    domain::identity::WorkbookDigest,
    runtime::{
        import::{import, ImportRequest},
        Runtime,
    },
};
use clap::Parser;
use std::{path::PathBuf, time::Instant};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    config: PathBuf,
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    sha256: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let runtime = Runtime::open(&args.config)?;
    let started = Instant::now();
    let outcome = async {
        let workbook = WorkbookDigest::parse(&args.sha256)?;
        let manifest = import(runtime.clone(), ImportRequest { original: args.input, workbook: workbook.clone() }).await?;
        let store = runtime.store.clone();
        let count = runtime.blocking(move || {
            let mut count = 0_u64;
            store.visit_source(&workbook, |_| {
                count = count.checked_add(1).ok_or(athletic_rust_pipeline::store::StoreError::CorruptData)?;
                Ok(())
            })?;
            Ok(count)
        }).await?;
        ensure!(count == manifest.stats.actual_data_rows, "source index count differs from parsed row count");
        let store = runtime.store.clone();
        let comparison_workbook = manifest.workbook.clone();
        let comparison_path = manifest.frozen.clone();
        let comparison = runtime.blocking(move || {
            let mut compared = 0_u64;
            let mut differing_rows = 0_u64;
            let mut differing_fields = 0_u64;
            let stats = athletic_rust_pipeline::workbook_ingest::visit_records(&comparison_path, |record| {
                let key = athletic_rust_pipeline::domain::identity::SourceRowKey::parse(&record.source_key)?;
                let previous = store.source_record(&comparison_workbook, &key)?.context("comparison row absent from original import")?;
                ensure!(record.sheet == previous.sheet && record.excel_row == previous.excel_row, "comparison row identity differs");
                ensure!(record.fields.keys().eq(previous.fields.keys()), "comparison field names differ");
                let differences = record.fields.iter().filter(|(name, value)| previous.fields.get(*name) != Some(*value)).count();
                if differences != 0 {
                    differing_rows += 1;
                    differing_fields += u64::try_from(differences)?;
                }
                compared += 1;
                Ok(())
            })?;
            ensure!(compared == stats.actual_data_rows, "comparison row count differs");
            Ok((compared, differing_rows, differing_fields))
        }).await?;
        athletic_rust_pipeline::runtime::snapshot::verify(&manifest.original, &manifest.workbook)?;
        println!("{}", serde_json::to_string(&serde_json::json!({
            "workbook_sha256": manifest.workbook,
            "source_rows": count,
            "sheets": manifest.stats.sheets.iter().map(|sheet| serde_json::json!({"name":sheet.name,"rows":sheet.actual_data_rows,"last_row":sheet.last_actual_row})).collect::<Vec<_>>(),
            "elapsed_seconds": started.elapsed().as_secs_f64(),
            "calamine_compared_rows": comparison.0,
            "calamine_differing_rows": comparison.1,
            "calamine_differing_fields": comparison.2,
            "original_hash_verified_after_import": true
        })).context("encoding aggregate import report")?);
        ensure!(comparison == (count, 0, 0), "Calamine field fidelity comparison failed; aggregate differences reported above");
        Ok(())
    }.await;
    runtime.drain().await?;
    outcome
}
