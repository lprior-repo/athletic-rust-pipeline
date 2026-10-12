use super::*;
use census_domain::school_directory::PssId;

#[test]
fn nces_name_only_or_conflicting_city_never_fills_a_postal_gap() -> TestResult {
    let entries = [entry(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "1 Main St",
    )?];
    let index = DirectoryIndex::build(&entries);
    for city in [None, Some("Columbus")] {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let (school, _) = CanonicalSchool::new(
            UsJurisdiction::Ohio,
            "Springfield High School",
            "springfield high school",
            city,
        );
        store.append(Table::Schools, &school)?;
        let (counts, outcomes) = process(
            &store,
            &index,
            &lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-10-04")),
            Mode::Apply,
        )?;
        println!(
            "city={city:?} linked={} no_match={}",
            counts.linked, counts.no_match
        );
        check!(eq; counts.linked, 0);
        check!(eq; counts.no_match, 1);
        let reason = if city.is_none() {
            "missing_city_corroboration"
        } else {
            "conflicting_city"
        };
        check!(eq; outcomes.first().and_then(|row| row.reason.as_deref()), Some(reason));
        check!(eq; outcomes.first().map(|row| row.postal_gap), Some(true));
        check!(eq; merged_school(&store)?.postal_addresses.len(), 0);
    }
    Ok(())
}

#[test]
fn pss_name_only_or_conflicting_city_never_links() -> TestResult {
    let public = entry(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "1 Main St",
    )?;
    let private = SchoolDirectoryEntry::identified(
        IdentifiedKey::Pss(PssId::parse("A2380006")?),
        SourceLabel::Pss,
        public.name().cloned(),
    )
    .with_address(public.address().cloned())
    .with_grades(public.grades());
    let index = DirectoryIndex::build(&[private]);
    for city in [None, Some("Columbus")] {
        let decision = index.link_name("Springfield High School", city, UsJurisdiction::Ohio, &[]);
        println!("PSS city={city:?} decision={decision:?}");
        check!(eq; decision, census_domain::school_directory::LinkDecision::NoMatch);
    }
    Ok(())
}

#[test]
fn ccd_and_pss_owned_keys_backfill_missing_city_with_their_own_provenance() -> TestResult {
    for (provider, key, label) in [
        (
            "nces-ccd",
            IdentifiedKey::Nces(NcesSchoolId::parse("390000000001")?),
            SourceLabel::Ccd,
        ),
        (
            "nces-pss",
            IdentifiedKey::Pss(PssId::parse("A2380006")?),
            SourceLabel::Pss,
        ),
    ] {
        let dir = tempfile::tempdir()?;
        let store = Store::open(dir.path())?;
        let (mut school, _) = CanonicalSchool::new(
            UsJurisdiction::Ohio,
            "Springfield High School",
            "springfield high school",
            None::<&str>,
        );
        let id = super::super::support::owner_id(&key).ok_or("missing key")?;
        school.source_identities.push(SourceIdentity::new(
            SourceNamespace::school_directory(provider, UsJurisdiction::Ohio),
            id,
        ));
        store.append(Table::Schools, &school)?;
        let public = entry(
            "390000000001",
            "Springfield High School",
            "Springfield",
            UsJurisdiction::Ohio,
            "1 Main St",
        )?;
        let directory =
            SchoolDirectoryEntry::identified(key, label.clone(), public.name().cloned())
                .with_address(public.address().cloned())
                .with_grades(public.grades());
        let index = DirectoryIndex::build(&[directory]);
        let lanes = lane_map(
            provider,
            Some("https://nces.ed.gov/source.csv"),
            Some("2026-10-04"),
        );
        let (counts, _) = process(&store, &index, &lanes, Mode::Apply)?;
        check!(eq; counts.postal_gaps, 1);
        check!(eq; counts.postal_filled, 1);
        check!(eq; counts.postal_remaining, 0);
        check!(eq; (counts.ccd_filled, counts.pss_filled), if provider == "nces-ccd" { (1, 0) } else { (0, 1) });
        let merged = merged_school(&store)?;
        let claim = merged.postal_addresses.first().ok_or("no postal fill")?;
        check!(eq; claim.source_label(), &label);
        check!(eq; claim.owner().namespace, SourceNamespace::school_directory(provider, UsJurisdiction::Ohio));
        check!(eq; claim.evidence().source.id.as_str(), provider);
        check!(eq; claim.evidence().source.url.as_deref(), Some("https://nces.ed.gov/source.csv"));
        check!(eq; claim.evidence().observed_on.as_str(), "2026-10-04");
        check!(eq; claim.capture_sha256(), CAPTURE_SHA);
        let (replay, _) = process(&store, &index, &lanes, Mode::Apply)?;
        check!(eq; (replay.postal_gaps, replay.postal_filled, replay.postal_remaining), (0, 0, 0));
    }
    Ok(())
}

#[test]
fn ccd_city_only_claim_is_an_enumerable_gap_not_a_postal_fill() -> TestResult {
    let (_dir, store) = store_with_school()?;
    let address = PostalAddress::of(
        None,
        None,
        Some(CityName::parse("Springfield")?),
        Some(UsJurisdiction::Ohio),
        None,
    )
    .ok_or("no address")?;
    let directory = SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse("390000000001")?),
        SourceLabel::Ccd,
        Some(SchoolName::parse("Springfield High School")?),
    )
    .with_address(Some(address));
    let index = DirectoryIndex::build(&[directory]);
    let (counts, outcomes) = process(
        &store,
        &index,
        &lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-10-04")),
        Mode::Apply,
    )?;
    check!(eq; counts.linked, 1);
    check!(eq; (counts.postal_gaps, counts.postal_filled, counts.postal_remaining, counts.ccd_filled), (1, 0, 1, 0));
    check!(eq; outcomes.len(), 1);
    check!(eq; outcomes.first().and_then(|row| row.reason.as_deref()), Some("no_published_street"));
    check!(eq; outcomes.first().map(|row| row.postal_gap), Some(true));
    Ok(())
}

#[test]
fn co_op_members_without_city_corroboration_remain_explicit_gaps() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.append(
        Table::Schools,
        &co_op_school(
            "Franklin County Coop",
            "Winchester",
            &["Centennial High School"],
        )?,
    )?;
    let index = DirectoryIndex::build(&[entry(
        "470000000001",
        "Centennial High School",
        "Franklin",
        UsJurisdiction::Tennessee,
        "1 Centennial Ln",
    )?]);
    let (counts, outcomes) = process(
        &store,
        &index,
        &lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-10-04")),
        Mode::Apply,
    )?;
    check!(eq; counts.linked, 0);
    check!(eq; counts.co_op_declined, 1);
    check!(eq; counts.postal_remaining, 1);
    check!(eq; outcomes.iter().any(|row| row.outcome == "co_op_declined"
        && row.reason.as_deref() == Some("conflicting_city") && row.postal_gap), true);
    check!(eq; merged_school(&store)?.postal_addresses.len(), 0);
    Ok(())
}
