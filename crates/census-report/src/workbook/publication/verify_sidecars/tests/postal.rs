use super::*;
use crate::export::postal::tests::{captured_school, SUMMARY_URL};

#[test]
fn source_backed_postal_sidecar_rejects_each_tampered_address_and_provenance_field() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let school = captured_school()?;
    store.append(Table::Schools, &school)?;
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        "Postal Sidecar Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "postal-sidecar"),
    );
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new(
            "synthetic_published_roster",
            Some("https://example.invalid/roster/postal-sidecar".to_string()),
        ),
    });
    store.append(Table::Athletes, &athlete)?;
    let options = Options {
        school_year: Some(SchoolYear::new(2026).ok_or("invalid fixture season")?),
        ..Options::default()
    };
    let workbook = crate::workbook::build(&store, &options)?;
    let generation = workbook.parent().ok_or("missing generation directory")?;
    let dataset = ExportDataset::reopen_frozen(&generation.join("frozen-input.json"))?;
    super::super::verify(generation, &dataset, &options)?;
    let path = generation.join("recruiting.csv");
    let original = std::fs::read(&path)?;
    let mut reader = csv::Reader::from_reader(original.as_slice());
    let headers = reader.headers()?.clone();
    let row = reader.records().next().ok_or("missing recruiting row")??;
    check!(eq; row.get(
        headers
            .iter()
            .position(|value| value == "postal_source_url")
            .ok_or("missing postal source URL column")?
    ),
    Some(SUMMARY_URL));
    for column in crate::export::postal::POSTAL_CSV_HEADERS {
        std::fs::write(&path, &original)?;
        rewrite_cell(&path, column, "forged postal provenance")?;
        let error = match super::super::verify(generation, &dataset, &options) {
            Err(error) => error.to_string(),
            Ok(()) => return Err(format!("forged postal column {column} accepted").into()),
        };
        check!(error.contains("recruiting.csv"), "{column}: {error}");
        check!(
            error.contains("forged postal provenance"),
            "{column}: {error}"
        );
    }
    Ok(())
}
