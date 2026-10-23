use crate::export::ExportDataset;
use crate::report::ReportError;
use census_domain::model::{CanonicalSchool, SchoolYear};
use census_domain::UsJurisdiction;
use census_store::Store;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn oversized_duplicate_school_review_expansion_is_refused_whole() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let mut dataset = ExportDataset::load(&store)?;
    for ordinal in 0..2_700 {
        let city = format!("City {ordinal}");
        let school = CanonicalSchool::new(
            UsJurisdiction::Wisconsin,
            "Same High",
            "same high",
            Some(&city),
        )
        .0;
        dataset.schools.insert(school.id.clone(), school);
    }
    check!(eq; dataset.schools.len(), 2_700);
    let year = SchoolYear::new(2026).ok_or("season")?;
    let result = super::super::super::retained::retained_records(&dataset, year, &[]);
    let error = match result {
        Err(ReportError::Grouping(error)) => error,
        Err(other) => return Err(format!("unexpected refusal: {other}").into()),
        Ok(_) => {
            return Err(
                "duplicate-school review expansion was materialized without admission".into(),
            )
        }
    };
    check!(matches!(
        error,
        crate::grouping::GroupingError::BudgetExceeded {
            stage: "conflicts",
            resource: "encoded bytes",
            available: 268_435_456,
            ..
        }
    ));
    Ok(())
}
