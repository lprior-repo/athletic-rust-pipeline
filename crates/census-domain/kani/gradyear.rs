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
    let school_year = SchoolYear(school_year);
    let grad_year = GradYear::of(grade, school_year);

    assert_eq!(
        grad_year.get(),
        school_year.start_year() + 13 - i16::from(grade.get()),
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
    let sy25 = SchoolYear(2025);
    assert!(GradYear::of(g11, sy25) == GradYear::CO2027);

    let g12 = Grade::new(12).expect("12 is a grade");
    let sy26 = SchoolYear(2026);
    assert!(GradYear::of(g12, sy26) == GradYear::CO2027);

    let g9 = Grade::new(9).expect("9 is a grade");
    assert!(GradYear::of(g9, sy25) == GradYear::new(2029).expect("2029 is in domain"));
}

/// Out-of-domain school years saturate instead of panicking, for every grade including 9..=12.
///
/// `school_year` is fully symbolic here, so this covers `i16::MIN`/`i16::MAX` observation years
/// arriving from a source's own header line.
#[kani::proof]
#[kani::unwind(16)]
fn check_gradyear_of_saturating() {
    let school_year: i16 = kani::any();
    let grade = Grade::new(kani::any::<u8>()).unwrap_or(Grade::new(9).expect("9 is a grade"));

    let grad_year = GradYear::of(grade, SchoolYear(school_year));

    assert_eq!(
        grad_year.get(),
        school_year
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
    let school_year = SchoolYear(school_year);
    let observed = ObservedGrade {
        grade,
        school_year,
        source: any_source_ref(),
    };

    let expected = GradYear::of(grade, school_year);
    assert!(observed.grad_year() == expected, "ObservedGrade::grad_year mismatch");
}
