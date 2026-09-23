//! Kani proof harnesses for `GradYear::of` cohort derivation and `ObservedGrade::grad_year`.

use crate::model::{Grade, GradYear, ObservedGrade, SchoolYear, SourceRef};

fn any_source_ref() -> SourceRef {
    SourceRef::new("kani", None)
}

/// The cohort formula holds exactly, and an in-domain observation always derives a `GradYear` that
/// `GradYear::new` accepts.
///
/// The premise is the school years a cohort can actually be observed in. The previous version of
/// this harness assumed `school_year` up to 2040 while claiming the derived class stays inside
/// `2020..=2040`; that claim is false (grade 9 in 2040 derives 2044) and CBMC produced exactly that
/// counterexample against the delivered harness. `of` is `start_year + 13 - grade`, so `y <= 2027`
/// with `grade in 9..=12` derives `2021..=2031`, which is what the two assertions below pin.
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
}

/// The known cohort anchors from the model docs.
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

/// Every season the domain admits derives its class by the saturating formula, for every grade.
///
/// A season now exists only through [`SchoolYear::new`], so the school year reaching
/// [`GradYear::of`] is always inside `MIN_START_YEAR..=MAX_START_YEAR`: the `i16::MIN`/`i16::MAX`
/// observation years this harness used to feed in cannot be constructed at all. The window is
/// quantified here and the constructor's refusal of the years outside it is asserted beside it,
/// which is what the saturated store was standing in for.
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
}

/// `ObservedGrade::grad_year` is exactly the cohort derivation of its own grade and school year.
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
}
