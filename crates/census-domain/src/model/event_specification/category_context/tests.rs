use super::*;
use crate::model::EventKind;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn cen10_published_subgroup_does_not_hide_contradictory_gender() -> TestResult {
    for (label, gender) in [
        ("Girls Varsity 100m", Gender::Boys),
        ("Girls U18 100m", Gender::Boys),
        ("Boys Varsity 100m", Gender::Girls),
        ("Boys U18 100m", Gender::Girls),
    ] {
        let specification = EventSpecification::from_published_label(label, &EventKind::Track100m)?;
        check!(eq;
            specification.category_in_context(gender, None),
            Err(SpecificationError::ConflictingSpecification),
            "contradictory published category: {label}"
        );
    }
    Ok(())
}

#[test]
fn cen10_compatible_published_gender_and_subgroup_remain_qualified() -> TestResult {
    for (label, gender) in [
        ("Girls Varsity 100m", Gender::Girls),
        ("Girls U18 100m", Gender::Girls),
        ("Boys Varsity 100m", Gender::Boys),
        ("Boys U18 100m", Gender::Boys),
    ] {
        let specification = EventSpecification::from_published_label(label, &EventKind::Track100m)?;
        check!(eq;
            specification.category_in_context(gender, None)?,
            specification.category,
            "compatible published category: {label}"
        );
    }
    Ok(())
}
