use crate::model::{Grade, GradYear, SchoolYear};

#[kani::proof]
#[kani::unwind(16)]
fn check_gradyear_of_formula() {
    let raw_grade: u8 = kani::any();
    let raw_year: i16 = kani::any();
    let (Some(grade), Some(school_year)) = (Grade::new(raw_grade), SchoolYear::new(raw_year)) else {
        return;
    };
    let implied = i32::from(school_year.get()) + 13 - i32::from(grade.get());
    let actual = GradYear::of(grade, school_year);
    if (i32::from(GradYear::MIN_YEAR)..=i32::from(GradYear::MAX_YEAR)).contains(&implied) {
        assert_eq!(actual.map(GradYear::get), i16::try_from(implied).ok());
    } else {
        assert_eq!(actual, None);
    }
    kani::cover!(implied == 2020 && actual.is_some(), "minimum supported cohort");
    kani::cover!(implied == 2040 && actual.is_some(), "maximum supported cohort");
    kani::cover!(implied == 2019 && actual.is_none(), "cohort below the minimum rejected");
    kani::cover!(implied == 2041 && actual.is_none(), "cohort above the maximum rejected");
    kani::cover!(raw_year == SchoolYear::MIN_START_YEAR, "minimum school year");
    kani::cover!(raw_year == SchoolYear::MAX_START_YEAR, "maximum school year");
    kani::cover!(raw_grade == 9, "minimum high-school grade");
    kani::cover!(raw_grade == 12, "maximum high-school grade");
}
