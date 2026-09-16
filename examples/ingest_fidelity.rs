use anyhow::{ensure, Context, Result};
use athletic_rust_pipeline::{
    domain::identity::WorkbookDigest, model::SourceRecord, runtime::snapshot, workbook_ingest, xlsx,
};
use clap::Parser;
use std::{collections::BTreeMap, io::Write, path::PathBuf, sync::mpsc, time::Instant};

#[derive(Parser)]
struct Args {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    sha256: String,
}

// Diagnostic classification only. Production ingestion uses Calamine, not these transformations.
fn difference_kind(old: &str, new: &str) -> &'static str {
    if old.replace("\r\n", "\n").replace('\r', "\n") == new {
        return "xml_line_endings";
    }
    if old.trim_matches([' ', '\t', '\r', '\n']) == new {
        return "xml_ascii_whitespace";
    }
    if old.contains("_x000D_") {
        let decoded = old.replace("_x000D_", "\r");
        if decoded == new {
            return "excel_carriage_return_escape";
        }
        if decoded.trim_matches([' ', '\t', '\r', '\n']) == new {
            return "escape_and_whitespace";
        }
    }
    if old
        .parse::<f64>()
        .is_ok_and(|value| value.is_finite() && value.to_string() == new)
    {
        return "numeric_lexical_format";
    }
    "unexplained"
}

fn main() -> Result<()> {
    let args = Args::parse();
    let expected = WorkbookDigest::parse(&args.sha256)?;
    snapshot::verify(&args.input, &expected)?;
    let started = Instant::now();
    let mut compared = 0_u64;
    let mut differing_rows = 0_u64;
    let mut differences = BTreeMap::<&str, u64>::new();
    let mut fields = BTreeMap::<String, u64>::new();
    let (baseline, library) = std::thread::scope(|scope| -> Result<_> {
        let (sender, receiver) = mpsc::sync_channel::<SourceRecord>(1);
        let input = &args.input;
        let baseline = scope.spawn(move || {
            xlsx::visit_records(input, |record| {
                sender
                    .send(record)
                    .map_err(|_| anyhow::anyhow!("comparison receiver closed"))
            })
        });
        let library = workbook_ingest::visit_records(&args.input, |record| {
            let old = receiver
                .recv()
                .context("baseline reader ended before library reader")?;
            ensure!(
                old.source_key == record.source_key
                    && old.sheet == record.sheet
                    && old.excel_row == record.excel_row,
                "reader source identity or order differs"
            );
            ensure!(
                old.fields.keys().eq(record.fields.keys()),
                "reader field names differ"
            );
            compared = compared
                .checked_add(1)
                .context("comparison count overflow")?;
            let mut different = false;
            for (name, value) in &record.fields {
                let previous = old.fields.get(name).context("baseline field missing")?;
                if previous != value {
                    different = true;
                    *differences
                        .entry(difference_kind(previous, value))
                        .or_default() += 1;
                    *fields.entry(name.clone()).or_default() += 1;
                }
            }
            if different {
                differing_rows = differing_rows
                    .checked_add(1)
                    .context("difference count overflow")?;
            }
            Ok(())
        });
        drop(receiver);
        let baseline = baseline
            .join()
            .map_err(|_| anyhow::anyhow!("baseline reader thread failed"))??;
        Ok((baseline, library?))
    })?;
    snapshot::verify(&args.input, &expected)?;
    let metadata_equal = serde_json::to_value(&baseline)? == serde_json::to_value(&library)?;
    let unexplained = differences
        .get("unexplained")
        .copied()
        .map_or(0, |count| count);
    let report = serde_json::json!({
        "source_sha256": expected, "compared_rows": compared, "differing_rows": differing_rows,
        "difference_categories": differences, "differing_field_counts": fields,
        "metadata_equal": metadata_equal, "total_seconds": started.elapsed().as_secs_f64(),
        "sheets": library.sheets.iter().map(|sheet| serde_json::json!({"name":sheet.name,"rows":sheet.actual_data_rows})).collect::<Vec<_>>(),
        "original_sha256_verified_before_and_after": true,
        "scope": "Reader semantic-difference diagnosis, not final workbook verification"
    });
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(&mut output, &report)?;
    writeln!(output)?;
    ensure!(
        compared == baseline.actual_data_rows && metadata_equal && unexplained == 0,
        "reader comparison contains unexplained differences; aggregate results emitted"
    );
    Ok(())
}
