use super::*;

#[test]
fn compound_throw_names_do_not_fabricate_or_discard_implement_context(
) -> Result<(), Box<dyn std::error::Error>> {
    for (label, kind) in [
        ("Discus Throw", EventKind::Discus),
        ("Javelin Throw", EventKind::Javelin),
        ("Hammer Throw", EventKind::Hammer),
    ] {
        let unspecified = EventSpecification::from_published_label(label, &kind)?;
        check!(eq; unspecified.implement, None, "{label}");
        let specified = EventSpecification::from_published_label(&format!("{label} 1kg"), &kind)?;
        check!(eq; specified.implement, Some(ImplementMass::try_from(1_000_000_000_u64)?), "{label}");
        for (invalid, error) in [
            ("not-a-mass", SpecificationError::InvalidLabel),
            ("0kg", SpecificationError::InvalidMass),
            ("1.0000000001kg", SpecificationError::InvalidLabel),
            ("1kg ignored", SpecificationError::InvalidLabel),
        ] {
            let result =
                EventSpecification::from_published_label(&format!("{label} {invalid}"), &kind);
            check!(eq; result, Err(error), "{label} {invalid}");
        }
    }
    Ok(())
}
