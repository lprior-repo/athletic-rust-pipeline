use super::*;
use crate::export::postal::tests::{captured_school, SUMMARY_URL};

#[test]
fn source_backed_postal_sidecar_rejects_each_tampered_address_and_provenance_field() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("store")).unwrap();
    let school = captured_school();
    store.append(Table::Schools, &school).unwrap();
    let athlete = CanonicalAthlete::new(
        &school.id,
        "Postal Sidecar Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "postal-sidecar"),
    );
    store.append(Table::Athletes, &athlete).unwrap();
    let options = Options {
        school_year: Some(SchoolYear::new(2026).unwrap()),
        ..Options::default()
    };
    let workbook = crate::workbook::build(&store, &options).unwrap();
    let generation = workbook.parent().unwrap();
    let dataset = ExportDataset::reopen_frozen(&generation.join("frozen-input.json")).unwrap();
    super::super::verify(generation, &dataset, &options).unwrap();
    let path = generation.join("recruiting.csv");
    let original = std::fs::read(&path).unwrap();
    let mut reader = csv::Reader::from_reader(original.as_slice());
    let headers = reader.headers().unwrap().clone();
    let row = reader.records().next().unwrap().unwrap();
    assert_eq!(
        row.get(
            headers
                .iter()
                .position(|value| value == "postal_source_url")
                .unwrap()
        ),
        Some(SUMMARY_URL)
    );
    for column in crate::export::postal::POSTAL_CSV_HEADERS {
        std::fs::write(&path, &original).unwrap();
        rewrite_cell(&path, column, "forged postal provenance");
        let error = super::super::verify(generation, &dataset, &options)
            .unwrap_err()
            .to_string();
        assert!(error.contains("recruiting.csv"), "{column}: {error}");
        assert!(
            error.contains("forged postal provenance"),
            "{column}: {error}"
        );
    }
}
