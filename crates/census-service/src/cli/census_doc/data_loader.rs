//! Data loading helpers: report parsing, seed counting, CSV reading.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};

use super::counted_seeds::counted_seeds;
use super::Seeds;

/// One CSV table: rows keyed by column header.
pub(super) type CsvTable = Vec<HashMap<String, String>>;

/// A report object keyed by name, as `serde_json` hands it over.
pub(super) type ReportObject = serde_json::Map<String, serde_json::Value>;

/// Everything the document builder needs: the report, the counted seeds, and the three tables.
pub(super) type LoadedData = (serde_json::Value, Seeds, CsvTable, CsvTable, CsvTable);

fn csv_rows(path: &Path) -> Result<CsvTable> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut reader = csv::Reader::from_path(path).with_context(|| format!("opening {path:?}"))?;
    let mut rows = Vec::new();
    for result in reader.deserialize() {
        let row: HashMap<String, String> =
            result.with_context(|| format!("deserializing row from {path:?}"))?;
        rows.push(row);
    }
    Ok(rows)
}

/// Totals extracted from the report's `totals` object.
pub(super) struct Totals {
    pub(super) total_schools: u64,
    pub(super) total_athletes: u64,
    pub(super) total_co2027: u64,
    pub(super) total_co2027_boys: u64,
    pub(super) total_co2027_girls: u64,
    pub(super) total_co2027_profile: u64,
    pub(super) total_co2027_grade: u64,
    pub(super) total_co2027_multi: u64,
    pub(super) total_coaches: u64,
    pub(super) total_coaches_email: u64,
    pub(super) meets_total: u64,
    pub(super) meets_an: u64,
}

/// Load report JSON, counted seeds, and all CSVs in one shot.
pub(super) fn load_all_data(store_out: &Path, data_dir: &Path) -> Result<LoadedData> {
    let report_path = store_out.join("report.json");
    let report_text = std::fs::read_to_string(&report_path)
        .with_context(|| format!("reading {report_path:?}"))?;
    let report: serde_json::Value =
        serde_json::from_str(&report_text).with_context(|| format!("parsing {report_path:?}"))?;

    let seeds = counted_seeds(store_out)?;
    let coaches = csv_rows(&data_dir.join("canonical-coaches.csv"))?;
    let co2027 = csv_rows(&data_dir.join("canonical-athletes-co2027.csv"))?;
    let recruiting = csv_rows(&data_dir.join("recruiting-co2027.csv"))?;

    Ok((report, seeds, coaches, co2027, recruiting))
}

/// Extract totals and key sub-objects from the report.
pub(super) fn extract_report_data(
    report: &serde_json::Value,
) -> (Totals, ReportObject, ReportObject, ReportObject) {
    let totals_obj = report
        .get("totals")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let by_state = report
        .get("by_state")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let schools_by_state = report
        .get("schools_by_state")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let meets = report
        .get("meets")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();

    let t = |key: &str| -> u64 { totals_obj.get(key).and_then(|v| v.as_u64()).unwrap_or(0) };
    let totals = Totals {
        total_schools: t("schools"),
        total_athletes: t("athletes"),
        total_co2027: t("class_of_2027"),
        total_co2027_boys: t("class_of_2027_boys"),
        total_co2027_girls: t("class_of_2027_girls"),
        total_co2027_profile: t("class_of_2027_with_profile_url"),
        total_co2027_grade: t("class_of_2027_with_grad_year_evidence"),
        total_co2027_multi: t("class_of_2027_multisource"),
        total_coaches: t("coaches"),
        total_coaches_email: t("coaches_with_email"),
        meets_total: meets.get("total").and_then(|v| v.as_u64()).unwrap_or(0),
        meets_an: meets
            .get("with_athletic_net_id")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
    };

    (totals, by_state, schools_by_state, meets)
}
