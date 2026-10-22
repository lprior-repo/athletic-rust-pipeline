use super::*;
use calamine::{open_workbook, Data, Reader, Xlsx};
use census_domain::model::{ProfileSelectionV1, SchoolYear};
use std::collections::BTreeMap;

#[tokio::test]
async fn genuine_reported_interpretations_match_workbook_csv_and_cold_readback() -> TestResult {
    for case in [CapturedCase::Michigan, CapturedCase::IllinoisBoys] {
        let fixture = Fixture::captured(case).await?;
        let selection =
            ProfileSelectionV1::new(fixture.store.store_identity()?, fixture.profile.id()?);
        let dataset = ExportDataset::for_job_request(
            &fixture.store,
            "genuine-reported-event-publication",
            &crate::export::ExportJobRequest::SelectedPublicationV1 { selection },
        )?;
        let options = crate::workbook::Options {
            grad_year: Some(2027),
            out: None,
            limit: None,
            scope: crate::report::Scope::AllSources,
            school_year: SchoolYear::new(2026).ok_or("school year")?,
            jurisdictions: fixture.profile.input().jurisdictions.clone(),
        };
        let censuses = crate::workbook::Censuses::of(
            &dataset,
            &fixture.store.out_dir(),
            &options.jurisdictions,
        )?;
        let workbook = crate::workbook::build_from(&dataset, &fixture.store, &options, &censuses)?;
        crate::workbook::publication::verify_published(&workbook)?;
        let generation = workbook.parent().ok_or("generation")?;
        let interpretations = csv_interpretations(&generation.join("reported_performances.csv"))?;
        check!(eq; interpretations.len(), 3);
        verify_cells(&workbook, &interpretations)?;
        check!(eq; csv::Reader::from_path(generation.join("best_marks.csv"))?.records().count(), 0);
        check!(eq; csv::Reader::from_path(generation.join("athletes.csv"))?.records().count(), 0);
        let manifest: Value =
            serde_json::from_slice(&std::fs::read(generation.join("manifest.json"))?)?;
        check!(eq; manifest.get("schema_revision").and_then(Value::as_u64), Some(10));
    }
    Ok(())
}

fn csv_interpretations(path: &std::path::Path) -> TestResult<BTreeMap<String, Value>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    let id = headers
        .iter()
        .position(|header| header == "Performance ID")
        .ok_or("CSV ID")?;
    let interpretation = headers
        .iter()
        .position(|header| header == "Reported Event Interpretation")
        .ok_or("CSV interpretation")?;
    reader.records().try_fold(BTreeMap::new(), |mut rows, row| {
        let row = row?;
        let text = row.get(interpretation).ok_or("interpretation")?;
        if !text.is_empty() {
            let previous =
                rows.insert(row.get(id).ok_or("ID")?.into(), serde_json::from_str(text)?);
            check!(previous.is_none());
        }
        Ok(rows)
    })
}

fn verify_cells(workbook: &std::path::Path, expected: &BTreeMap<String, Value>) -> TestResult {
    let mut book: Xlsx<_> = open_workbook(workbook)?;
    let sheet = book.worksheet_range("PRs")?;
    let headers = sheet.rows().next().ok_or("PR headers")?;
    let column = |name: &str| {
        headers
            .iter()
            .position(|header| header.to_string() == name)
            .ok_or_else(|| format!("column {name}"))
    };
    let id = column("Performance ID")?;
    let status = column("Verification Status")?;
    let interpretation = column("Reported Event Interpretation")?;
    let mut found = BTreeMap::new();
    for row in sheet.rows().skip(1).filter(|row| {
        row.get(status).is_some_and(|cell| {
            cell.to_string() == "reported_event_context_needs_review_not_a_best"
        })
    }) {
        let performance = row.get(id).ok_or("PR ID")?.to_string();
        let value: Value = serde_json::from_str(
            &row.get(interpretation)
                .ok_or("PR interpretation")?
                .to_string(),
        )?;
        check!(eq; expected.get(&performance), Some(&value));
        for unknown in [
            "Calculated PR",
            "School at performance",
            "State at performance",
            "Current school",
            "Current state",
            "Current school as-of",
        ] {
            check!(row
                .get(column(unknown)?)
                .is_none_or(|cell| matches!(cell, Data::Empty)));
        }
        check!(!row
            .get(column("Published mark")?)
            .ok_or("published mark")?
            .to_string()
            .is_empty());
        found.insert(performance, value);
    }
    check!(eq; &found, expected);
    Ok(())
}
