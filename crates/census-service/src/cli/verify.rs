//! `census-service verify`: check workbook data rows against the store.
//!
//! The seal proves that meta-sheet counts agree with the store. Verify goes further: it reads the
//! Athletes and Performances sheets, maps columns by header name (never by position), samples at
//! most 5 000 rows, and asserts that each sampled row exists in the store with matching data.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use calamine::Reader;
use clap::Args;

use census_store::Store;
use census_reconcile::verify::{
    self, column_index, missing_columns, sheets_matching_prefix, verify_athletes,
    verify_performances, ATHLETES_REQUIRED, PERFORMANCES_REQUIRED,
};

/// `census-service verify`
#[derive(Debug, Args)]
pub struct VerifyArgs {
    /// The workbook to verify. Defaults to the newest `out/*.xlsx`.
    #[arg(long)]
    pub workbook: Option<PathBuf>,

    /// Sampling stride: check every k-th data row. Larger values check fewer rows.
    /// At most 5 000 samples per sheet regardless of k.
    #[arg(long, default_value_t = 10)]
    pub sample_every: usize,

    /// Graduation year used to locate the newest workbook in `out/` (for year-prefixed names).
    #[arg(long, default_value_t = 2027)]
    pub grad_year: i16,
}

/// Find the newest `.xlsx` file in the store's `out` directory.
fn find_workbook(store: &Store, _args: &VerifyArgs) -> Result<PathBuf> {
    let out_dir = store.out_dir();
    let mut candidates: Vec<PathBuf> = Vec::new();

    for entry in std::fs::read_dir(&out_dir)
        .with_context(|| format!("reading output directory {}", out_dir.display()))?
    {
        let entry = entry.with_context(|| format!("listing entry in {}", out_dir.display()))?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "xlsx") {
            candidates.push(path);
        }
    }

    candidates.sort_by(|a, b| {
        let a_time = a.metadata().ok().and_then(|m| m.modified().ok());
        let b_time = b.metadata().ok().and_then(|m| m.modified().ok());
        b_time.cmp(&a_time)
    });

    candidates
        .first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no .xlsx workbook found in {}", out_dir.display()))
}

/// Read all sheets from the workbook into a map of name → rows.
fn read_workbook_sheets(path: &std::path::Path) -> Result<HashMap<String, Vec<Vec<String>>>> {
    let mut book = calamine::open_workbook_auto(path)
        .with_context(|| format!("opening {}", path.display()))?;

    let mut sheets = HashMap::new();
    for sheet_name in book.sheet_names() {
        let range = book
            .worksheet_range(&sheet_name)
            .with_context(|| format!("reading sheet {sheet_name}"))?;

        let rows: Vec<Vec<String>> = range
            .rows()
            .map(|row| {
                row.iter()
                    .map(|cell| match cell {
                        calamine::Data::String(text) => text.clone(),
                        calamine::Data::Empty => String::new(),
                        other => other.to_string(),
                    })
                    .collect()
            })
            .collect();

        sheets.insert(sheet_name.to_string(), rows);
    }

    Ok(sheets)
}

/// Verify the Athletes sheet: validate headers, map columns, sample, check against store.
fn verify_athletes_sheet(
    sheets: &HashMap<String, Vec<Vec<String>>>,
    store: &Store,
    sample_every: usize,
) -> Result<(usize, census_reconcile::verify::EntityCheck)> {
    if !sheets.contains_key("Athletes") {
        bail!("workbook has no 'Athletes' sheet");
    }

    let athletes_rows = sheets
        .get("Athletes")
        .ok_or_else(|| anyhow::anyhow!("'Athletes' sheet is missing"))?;

    let athletes_headers = athletes_rows
        .first()
        .ok_or_else(|| anyhow::anyhow!("'Athletes' sheet has no header row"))?;

    let athletes_missing = missing_columns(athletes_headers, ATHLETES_REQUIRED);
    if !athletes_missing.is_empty() {
        bail!(
            "'Athletes' sheet missing required columns: {}",
            athletes_missing.join(", ")
        );
    }

    let athletes_col_map: HashMap<&str, usize> = ATHLETES_REQUIRED
        .iter()
        .filter_map(|&name| column_index(athletes_headers, name).map(|i| (name, i)))
        .collect();

    let athletes_data = athletes_rows
        .get(1..)
        .ok_or_else(|| anyhow::anyhow!("'Athletes' sheet has no header row"))?;
    let athletes_total = athletes_data.len();
    let athletes_sampled = verify::sample_indices(athletes_total, sample_every);

    let athletes_check =
        verify_athletes(store, athletes_data, &athletes_sampled, &athletes_col_map)
            .map_err(|d| anyhow::anyhow!("athletes verification failed: {}", d.message))?;

    Ok((athletes_total, athletes_check))
}

/// Verify the Performances sheets: validate headers, map columns, sample, check against store.
fn verify_performances_sheets(
    sheets: &HashMap<String, Vec<Vec<String>>>,
    store: &Store,
    sample_every: usize,
) -> Result<(usize, census_reconcile::verify::EntityCheck)> {
    let perf_rows = sheets_matching_prefix(sheets, "Performances_");
    let perf_headers = perf_rows
        .first()
        .ok_or_else(|| anyhow::anyhow!("Performances sheet has no header row"))?;

    let perf_missing = missing_columns(perf_headers, PERFORMANCES_REQUIRED);
    if !perf_missing.is_empty() {
        bail!(
            "'Performances' sheet missing required columns: {}",
            perf_missing.join(", ")
        );
    }

    let perf_col_map: HashMap<&str, usize> = PERFORMANCES_REQUIRED
        .iter()
        .filter_map(|&name| column_index(perf_headers, name).map(|i| (name, i)))
        .collect();

    let perf_data = perf_rows
        .get(1..)
        .ok_or_else(|| anyhow::anyhow!("Performances sheet has no header row"))?;
    let perf_total = perf_data.len();
    let perf_sampled = verify::sample_indices(perf_total, sample_every);

    let perf_check = verify_performances(store, perf_data, &perf_sampled, &perf_col_map)
        .map_err(|d| anyhow::anyhow!("performances verification failed: {}", d.message))?;

    Ok((perf_total, perf_check))
}

/// Run the verification: read workbook sheets, sample rows, and check each against the store.
pub fn run_verify(store: &Store, args: &VerifyArgs) -> Result<()> {
    let workbook_path = match &args.workbook {
        Some(path) => path.clone(),
        None => find_workbook(store, args)?,
    };

    if !workbook_path.exists() {
        bail!("workbook not found: {}", workbook_path.display());
    }

    let sheets = read_workbook_sheets(&workbook_path)?;

    let (athletes_total, athletes_check) =
        verify_athletes_sheet(&sheets, store, args.sample_every)?;

    let (perf_total, perf_check) = verify_performances_sheets(&sheets, store, args.sample_every)?;

    let ok = athletes_check.passed == athletes_check.sampled_indices.len()
        && perf_check.passed == perf_check.sampled_indices.len();

    if ok {
        println!(
            "verify: OK ({} athletes sampled of {} rows, {} performances sampled of {} rows)",
            athletes_check.passed, athletes_total, perf_check.passed, perf_total
        );
    } else {
        println!("verify: FAILED");
        bail!("verification failed");
    }

    Ok(())
}
