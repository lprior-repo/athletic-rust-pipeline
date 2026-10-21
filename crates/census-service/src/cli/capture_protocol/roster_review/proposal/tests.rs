use super::{provider_subjects, ProviderOwnerError, SourceIdentity, SourceNamespace};

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn school_or_program_with_distinct_provider_owners_refuses_proposal() -> TestResult {
    for conflicting_school in [true, false] {
        let mut schools = vec![SourceIdentity::new(
            SourceNamespace::MilesplitSchool,
            "12439",
        )];
        let mut programs = vec![SourceIdentity::new(SourceNamespace::MilesplitTeam, "12439")];
        if conflicting_school {
            schools.push(SourceIdentity::new(
                SourceNamespace::MilesplitSchool,
                "99999",
            ));
        } else {
            programs.push(SourceIdentity::new(SourceNamespace::MilesplitTeam, "99999"));
        }
        let error = provider_subjects(&schools, &programs)
            .err()
            .ok_or("ambiguous provider owner admitted")?;
        check!(matches!(
            error.downcast_ref::<ProviderOwnerError>(),
            Some(ProviderOwnerError::Ambiguous)
        ));
    }
    Ok(())
}

#[test]
fn school_and_program_with_different_provider_owners_refuses_proposal() -> TestResult {
    let schools = [SourceIdentity::new(
        SourceNamespace::MilesplitSchool,
        "12439",
    )];
    let programs = [SourceIdentity::new(SourceNamespace::MilesplitTeam, "99999")];
    let error = provider_subjects(&schools, &programs)
        .err()
        .ok_or("foreign program owner admitted")?;
    check!(matches!(
        error.downcast_ref::<ProviderOwnerError>(),
        Some(ProviderOwnerError::Mismatch)
    ));
    Ok(())
}

mod retained;

mod owned;
