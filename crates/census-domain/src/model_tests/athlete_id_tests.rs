use super::*;

#[test]
fn athlete_ids_separate_gender_sides_and_ignore_spacing() -> Result<(), Box<dyn std::error::Error>>
{
    let school = CanonicalSchool::mint(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford", None);
    let cohort = GradYear::CO2027;
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169");
    let mint = |name: &str, gender| CanonicalAthlete::mint(&school, name, cohort, gender, &source);
    let boys = mint("Julian Aguilera", Gender::Boys);
    check!(eq; boys, mint("  Julian   Aguilera  ", Gender::Boys));
    let girls = mint("Julian Aguilera", Gender::Girls);
    let other = mint("Julian Aguilera", Gender::Mixed);
    check!(ne; other, mint("Julian Aguilera", Gender::Unknown));
    check!(ne; boys, girls);
    check!(ne; boys, other);
    check!(ne; girls, other);
    Ok(())
}

#[test]
fn candidate_key_equality_agrees_with_the_retained_athlete_key(
) -> Result<(), Box<dyn std::error::Error>> {
    let school = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Madison West High School",
        "madison-west",
        None,
    );
    let class = GradYear::CO2027;
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169");
    let raw = CanonicalAthlete::new(&school, "Jane Doe", class, Gender::Girls, source.clone())?;
    let respelled =
        CanonicalAthlete::new(&school, "jane  DOE", class, Gender::Girls, source.clone())?;
    check!(eq; raw.id, respelled.id);
    check!(eq; raw.candidate_key(), respelled.candidate_key());
    check!(raw.same_natural_key(&respelled));

    let mixed = CanonicalAthlete::new(&school, "Jane Doe", class, Gender::Mixed, source.clone())?;
    let unknown =
        CanonicalAthlete::new(&school, "Jane Doe", class, Gender::Unknown, source.clone())?;
    check!(ne; mixed.id, unknown.id);
    check!(ne; mixed.candidate_key(), unknown.candidate_key());
    check!(!mixed.same_natural_key(&unknown));
    Ok(())
}

#[test]
fn candidate_index_collisions_do_not_erase_distinct_categories(
) -> Result<(), Box<dyn std::error::Error>> {
    let school = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Madison West High School",
        "madison-west",
        None,
    );
    let key = |gender| AthleteCandidateKey::new(&school, "Jane Doe", GradYear::CO2027, gender);
    let mixed = key(Gender::Mixed);
    let unknown = key(Gender::Unknown);
    check!(eq; mixed.index_id(), unknown.index_id());
    let keys = std::collections::BTreeSet::from([mixed.clone(), unknown.clone()]);
    check!(eq; keys.len(), 2);
    check!(keys.contains(&mixed));
    check!(keys.contains(&unknown));
    Ok(())
}

#[test]
fn athlete_natural_ownership_follows_namespace_and_provider_id_not_locator(
) -> Result<(), Box<dyn std::error::Error>> {
    let school = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Madison West High School",
        "madison-west",
        None,
    );
    let class = GradYear::CO2027;
    let owned = |url: Option<&str>| -> Result<CanonicalAthlete, Box<dyn std::error::Error>> {
        let mut identity = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111");
        identity.url = url.map(str::to_string);
        Ok(CanonicalAthlete::new(
            &school,
            "Jane Doe",
            class,
            Gender::Girls,
            identity,
        )?)
    };
    let base = owned(Some("https://milesplit.com/athletes/111"))?;
    let www = owned(Some("https://www.milesplit.com/athletes/111"))?;
    let slug = owned(Some("https://www.milesplit.com/athletes/111-jane-doe"))?;
    let absent = owned(None)?;
    for variant in [&www, &slug, &absent] {
        check!(eq; base.id, variant.id);
        check!(base.same_natural_key(variant));
    }
    let other_id = CanonicalAthlete::new(
        &school,
        "Jane Doe",
        class,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "112"),
    )?;
    let other_namespace = CanonicalAthlete::new(
        &school,
        "Jane Doe",
        class,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::TfrrsAthlete, "111"),
    )?;
    let other_school = CanonicalAthlete::new(
        &CanonicalSchool::mint(
            UsJurisdiction::Minnesota,
            "Other High School",
            "other",
            None,
        ),
        "Jane Doe",
        class,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111"),
    )?;
    for mismatch in [&other_id, &other_namespace, &other_school] {
        check!(!base.same_natural_key(mismatch));
    }
    Ok(())
}
