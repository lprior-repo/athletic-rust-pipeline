
use crate::model::{Grade, GradYear, ObservedGrade, SchoolYear, SourceRef};

fn any_source_ref() -> SourceRef {
    SourceRef::new("kani", None)
}

#[kani::proof]
#[kani::unwind(16)]
fn check_gradyear_of_formula() {
    let grade: u8 = kani::any();
    let school_year: i16 = kani::any();

    kani::assume((9..=12).contains(&grade));
    kani::assume((2020..=2027).contains(&school_year));

    let grade = Grade::new(grade).expect("grade is in 9..=12");
    let school_year = SchoolYear::new(school_year).expect("2020..=2027 is a season");
    let grad_year = GradYear::of(grade, school_year);

    assert_eq!(
        grad_year.get(),
        school_year.get() + 13 - i16::from(grade.get()),
        "GradYear::of must be start_year + 13 - grade"
    );
    assert!(
        GradYear::new(grad_year.get()).is_some(),
        "in-domain observation derived {grad_year}, which GradYear::new rejects"
    );

    kani::cover!(grade.get() == 9, "grade boundary 9 is reachable");
    kani::cover!(grade.get() == 12, "grade boundary 12 is reachable");
    kani::cover!(school_year.get() == 2020, "school_year lower bound is reachable");
    kani::cover!(school_year.get() == 2027, "school_year upper bound is reachable");
}

#[kani::proof]
#[kani::unwind(16)]
fn check_gradyear_of_known_values() {
    let g11 = Grade::new(11).expect("11 is a grade");
    let sy25 = SchoolYear::new(2025).expect("2025 is a season");
    assert!(GradYear::of(g11, sy25) == GradYear::CO2027);

    let g12 = Grade::new(12).expect("12 is a grade");
    let sy26 = SchoolYear::new(2026).expect("2026 is a season");
    assert!(GradYear::of(g12, sy26) == GradYear::CO2027);

    let g9 = Grade::new(9).expect("9 is a grade");
    assert!(GradYear::of(g9, sy25) == GradYear::new(2029).expect("2029 is in domain"));
}

#[kani::proof]
#[kani::unwind(16)]
fn check_gradyear_of_saturating() {
    let school_year: i16 = kani::any();
    let grade = Grade::new(kani::any::<u8>()).unwrap_or(Grade::new(9).expect("9 is a grade"));

    kani::assume((SchoolYear::MIN_START_YEAR..=SchoolYear::MAX_START_YEAR).contains(&school_year));
    let school_year = SchoolYear::new(school_year).expect("the year is inside the season window");

    assert!(
        SchoolYear::new(SchoolYear::MIN_START_YEAR - 1).is_none()
            && SchoolYear::new(SchoolYear::MAX_START_YEAR + 1).is_none(),
        "the season constructor must refuse a year outside its window"
    );

    let grad_year = GradYear::of(grade, school_year);

    assert_eq!(
        grad_year.get(),
        school_year
            .get()
            .saturating_add(13)
            .saturating_sub(i16::from(grade.get())),
        "saturating arithmetic in GradYear::of changed"
    );

    kani::cover!(
        school_year.get() == SchoolYear::MIN_START_YEAR,
        "min season boundary is reachable"
    );
    kani::cover!(
        school_year.get() == SchoolYear::MAX_START_YEAR,
        "max season boundary is reachable"
    );
}

#[kani::proof]
#[kani::unwind(16)]
fn check_observed_grade_grad_year() {
    let grade: u8 = kani::any();
    let school_year: i16 = kani::any();

    kani::assume((9..=12).contains(&grade));
    kani::assume((2020..=2040).contains(&school_year));

    let grade = Grade::new(grade).expect("grade is in 9..=12");
    let school_year = SchoolYear::new(school_year).expect("2020..=2040 is a season");
    let observed = ObservedGrade {
        grade,
        school_year,
        source: any_source_ref(),
    };

    let expected = GradYear::of(grade, school_year);
    assert!(observed.grad_year() == expected, "ObservedGrade::grad_year mismatch");

    kani::cover!(grade.get() == 9, "grade boundary 9 is reachable");
    kani::cover!(grade.get() == 12, "grade boundary 12 is reachable");
    kani::cover!(school_year.get() == 2020, "school_year lower bound is reachable");
    kani::cover!(school_year.get() == 2040, "school_year upper bound is reachable");
}
