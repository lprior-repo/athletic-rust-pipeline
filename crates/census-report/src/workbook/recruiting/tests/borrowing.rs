use super::*;

fn assert_borrowed(projection: &Recruiting<'_>, derivation: &Derivation<'_>) -> TestResult {
    check!(std::ptr::eq(
        projection.dataset.athletes,
        derivation.athletes()
    ));
    check!(eq; projection.dataset.schools.len(), derivation.schools().len());
    for school in derivation.schools() {
        let (key, borrowed) = projection
            .dataset
            .schools
            .get_key_value(school.id.as_str())
            .ok_or("borrowed school is missing")?;
        check!(std::ptr::eq(*borrowed, school));
        check!(std::ptr::eq(*key, school.id.as_str()));
    }
    Ok(())
}

#[test]
fn recruiting_borrows_selected_population_and_school_records() -> TestResult {
    let fixture = fixture()?;
    let dataset = crate::export::ExportDataset::load(&fixture.store)?;
    let derivation = Derivation::of(&dataset, Scope::Core, Some(2027), &[])?;
    let projection = recruiting(&derivation)?;
    check!(eq; dataset.athletes.len(), 3);
    check!(eq; projection.dataset.athletes.len(), 2);
    check!(!projection.dataset.schools.is_empty());
    assert_borrowed(&projection, &derivation)?;
    let empty = Derivation::of(&dataset, Scope::Core, Some(2030), &[])?;
    let projection = recruiting(&empty)?;
    check!(projection.dataset.athletes.is_empty());
    assert_borrowed(&projection, &empty)
}

#[test]
fn recruiting_borrows_the_accepted_alias_cohort_union() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let school = school(&store, UsJurisdiction::Wisconsin, "Borrowed Alias High")?;
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9001001")
        .with_url("https://wi.milesplit.com/athletes/9001001");
    let mut members = [
        CanonicalAthlete::new(
            &school,
            "Borrowed Alias Runner",
            GradYear::CO2027,
            Gender::Boys,
            source.clone(),
        )?,
        CanonicalAthlete::new(
            &school,
            "B. Alias Runner",
            GradYear::CO2027,
            Gender::Boys,
            source,
        )?,
    ];
    for member in &mut members {
        member.evidence = evidence("wiaa_results", Some("https://wiaa.test/borrowed-alias"));
    }
    members.sort_by(|left, right| left.id.cmp(&right.id));
    let [representative, alias] = &mut members;
    publish_fixture_cohort(alias, "wiaa_results", "borrowed-alias", DAY);
    check!(representative.published_graduations.is_empty());
    check!(eq; alias.published_graduations.len(), 1);
    store.append_many(Table::Athletes, &members)?;
    identity::accept(&store, &members)?;
    let dataset = crate::export::ExportDataset::load(&store)?;
    check!(eq; dataset.athletes.len(), 2);
    check!(eq; dataset.canonical_aliases.len(), 1);
    let derivation = Derivation::of(&dataset, Scope::AllSources, Some(2027), &[])?;
    let projection = recruiting(&derivation)?;
    let [athlete] = projection.dataset.athletes else {
        return Err("borrowed accepted alias union lost its single selected athlete".into());
    };
    let [representative, alias] = &members;
    check!(eq; athlete.id, representative.id);
    check!(eq; athlete.published_graduations, alias.published_graduations);
    check!(eq; athlete.derived_cohort_confidence(),
        Some(census_domain::model::Confidence::HIGH));
    assert_borrowed(&projection, &derivation)
}
