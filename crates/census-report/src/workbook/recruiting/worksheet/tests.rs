use super::{observed_grade, Cell};
use census_domain::model::{
    CanonicalAthlete, Gender, GradYear, ObservedGrade, SchoolId, SchoolYear, SourceIdentity,
    SourceNamespace, SourceRef,
};

fn make_athlete() -> Result<CanonicalAthlete, Box<dyn std::error::Error>> {
    let school = SchoolId::mint("sch", &["synthetic school"]);
    let identity = SourceIdentity::new(SourceNamespace::Other("fixture".into()), "runner");
    Ok(CanonicalAthlete::new(
        &school,
        "Synthetic runner",
        GradYear::CO2027,
        Gender::Boys,
        identity,
    )?)
}

fn grade(grade: u8, year: i16) -> Result<ObservedGrade, Box<dyn std::error::Error>> {
    let g = census_domain::model::Grade::new(grade).ok_or("invalid grade")?;
    let y = SchoolYear::new(year).ok_or("invalid school year")?;
    Ok(ObservedGrade {
        grade: g,
        school_year: y,
        source: SourceRef::new("fixture", None),
    })
}

#[test]
fn no_observed_grades_yields_empty_cells() -> Result<(), Box<dyn std::error::Error>> {
    let athlete = make_athlete()?;
    let (grade_cell, year_cell) = observed_grade(&athlete)?;
    check!(matches!(grade_cell, Cell::Empty));
    check!(matches!(year_cell, Cell::Empty));
    Ok(())
}

#[test]
fn observed_grade_selects_latest_year_and_deduplicates() -> Result<(), Box<dyn std::error::Error>> {
    let mut athlete = make_athlete()?;
    athlete.observed_grades.push(grade(10, 2024)?);
    athlete.observed_grades.push(grade(11, 2025)?);
    athlete.observed_grades.push(grade(12, 2025)?);
    athlete.observed_grades.push(grade(11, 2025)?);
    let before = serde_json::to_vec(&athlete)?;
    let expected = (
        Cell::text("11 | 12"),
        Cell::text(SchoolYear::new(2025).ok_or("year")?.short()),
    );
    check!(eq; observed_grade(&athlete)?, expected);
    athlete.observed_grades.reverse();
    check!(eq; observed_grade(&athlete)?, expected);
    athlete.observed_grades.reverse();
    check!(eq; serde_json::to_vec(&athlete)?, before);
    Ok(())
}
