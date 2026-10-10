use super::*;

#[test]
fn athlete_ids_separate_gender_sides_and_ignore_spacing() {
    let school = CanonicalSchool::mint(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford", None);
    let cohort = GradYear::CO2027;
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169");
    let mint = |name: &str, gender| CanonicalAthlete::mint(&school, name, cohort, gender, &source);
    let boys = mint("Julian Aguilera", Gender::Boys);
    assert_eq!(boys, mint("  Julian   Aguilera  ", Gender::Boys));
    let girls = mint("Julian Aguilera", Gender::Girls);
    let other = mint("Julian Aguilera", Gender::Mixed);
    assert_ne!(other, mint("Julian Aguilera", Gender::Unknown));
    assert_ne!(boys, girls);
    assert_ne!(boys, other);
    assert_ne!(girls, other);
}

#[test]
fn candidate_key_equality_agrees_with_the_retained_athlete_key() {
    let school = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Madison West High School",
        "madison-west",
        None,
    );
    let class = GradYear::CO2027;
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169");
    let raw = CanonicalAthlete::new(&school, "Jane Doe", class, Gender::Girls, source.clone());
    let respelled =
        CanonicalAthlete::new(&school, "jane  DOE", class, Gender::Girls, source.clone());
    assert_eq!(raw.id, respelled.id);
    assert_eq!(raw.candidate_key(), respelled.candidate_key());
    assert!(raw.same_natural_key(&respelled));

    let mixed = CanonicalAthlete::new(&school, "Jane Doe", class, Gender::Mixed, source.clone());
    let unknown =
        CanonicalAthlete::new(&school, "Jane Doe", class, Gender::Unknown, source.clone());
    assert_ne!(mixed.id, unknown.id);
    assert_ne!(mixed.candidate_key(), unknown.candidate_key());
    assert!(!mixed.same_natural_key(&unknown));
}

#[test]
fn candidate_index_collisions_do_not_erase_distinct_categories() {
    let school = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Madison West High School",
        "madison-west",
        None,
    );
    let key = |gender| AthleteCandidateKey::new(&school, "Jane Doe", GradYear::CO2027, gender);
    let mixed = key(Gender::Mixed);
    let unknown = key(Gender::Unknown);
    assert_eq!(mixed.index_id(), unknown.index_id());
    let keys = std::collections::BTreeSet::from([mixed.clone(), unknown.clone()]);
    assert_eq!(keys.len(), 2);
    assert!(keys.contains(&mixed));
    assert!(keys.contains(&unknown));
}

#[test]
fn athlete_natural_ownership_follows_namespace_and_provider_id_not_locator() {
    let school = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Madison West High School",
        "madison-west",
        None,
    );
    let class = GradYear::CO2027;
    let owned = |url: Option<&str>| {
        let mut identity = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111");
        identity.url = url.map(str::to_string);
        CanonicalAthlete::new(&school, "Jane Doe", class, Gender::Girls, identity)
    };
    let base = owned(Some("https://milesplit.com/athletes/111"));
    let www = owned(Some("https://www.milesplit.com/athletes/111"));
    let slug = owned(Some("https://www.milesplit.com/athletes/111-jane-doe"));
    let absent = owned(None);
    for variant in [&www, &slug, &absent] {
        assert_eq!(base.id, variant.id);
        assert!(base.same_natural_key(variant));
    }
    let other_id = CanonicalAthlete::new(
        &school,
        "Jane Doe",
        class,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "112"),
    );
    let other_namespace = CanonicalAthlete::new(
        &school,
        "Jane Doe",
        class,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::TfrrsAthlete, "111"),
    );
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
    );
    for mismatch in [&other_id, &other_namespace, &other_school] {
        assert!(!base.same_natural_key(mismatch));
    }
}
