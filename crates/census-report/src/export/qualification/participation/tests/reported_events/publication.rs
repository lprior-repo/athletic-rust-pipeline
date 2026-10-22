use super::*;
use calamine::{open_workbook, Reader, Xlsx};
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
        check!(eq; manifest.get("schema_revision").and_then(Value::as_u64), Some(12));
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
    check!(eq; book.sheet_names(), vec!["Athletes".to_owned()]);
    let sheet = book.worksheet_range("Athletes")?;
    let headers = sheet.rows().next().ok_or("athlete headers")?;
    let column = |name: &str| {
        headers
            .iter()
            .position(|header| header == name)
            .ok_or_else(|| format!("column {name}"))
    };
    let mut found = BTreeMap::new();
    for row in sheet.rows().skip(1) {
        for (label, expected) in [
            ("Record Basis", "source_reported"),
            ("Verification Status", "reported_unverified_needs_review"),
            ("Current school", "Unknown"),
            ("Current state", "Unknown"),
            ("Current school as-of", ""),
            ("Observed bests", ""),
        ] {
            check!(eq; row.get(column(label)?).ok_or("athlete cell")?.to_string(), expected);
        }
        let unresolved = row
            .get(column("Unresolved results")?)
            .ok_or("unresolved results")?
            .to_string();
        for line in unresolved.lines() {
            check!(line.starts_with("reported_event_context_needs_review_not_a_best; "));
            let id = line
                .split_once("; Performance ID: ")
                .ok_or("performance ID")?
                .1
                .split(';')
                .next()
                .ok_or("performance identifier")?;
            let interpretation = line
                .split_once("; Reported Event Interpretation: ")
                .ok_or("interpretation")?
                .1;
            let value: Value = serde_json::from_str(interpretation)?;
            check!(eq; expected.get(id), Some(&value));
            for absent in ["; Calculated PR: ", "; School: ", "; State: "] {
                check!(!line.contains(absent));
            }
            let mark = line
                .split_once("; Published mark: ")
                .ok_or("published mark")?
                .1
                .split(';')
                .next()
                .ok_or("published mark text")?;
            check!(!mark.is_empty());
            check!(found.insert(id.to_owned(), value).is_none());
        }
    }
    check!(eq; &found, expected);
    Ok(())
}
