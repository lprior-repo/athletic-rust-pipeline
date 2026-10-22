use super::*;

fn include_profile(profile: &QueryProfileV1) -> TestResult<QueryProfileV1> {
    let mut raw = profile.input().clone();
    raw.include_provisional = ProvisionalInclusionV1::Include;
    Ok(QueryProfileV1::new(raw)?)
}

#[test]
fn explicit_opt_in_preserves_archived_roster_and_identity_requirements() -> TestResult {
    for accepted_identity in [true, false] {
        let mut fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", accepted_identity)?;
        let exclude = fixture.profile.clone();
        let include = include_profile(&exclude)?;
        fixture.store.register_profile(&include)?;
        for profile in [exclude, include] {
            fixture.profile = profile;
            let snapshot = fixture.store.snapshot();
            let dataset = fixture.dataset(&snapshot)?;
            let before = serde_json::to_vec(&dataset)?;
            let verified = PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified();
            check!(eq; verified.profile_id(), &fixture.profile.id()?);
            if accepted_identity {
                check!(eq; verified.resolution(&fixture.athlete).school(), Some(&fixture.school));
            } else {
                check!(eq; verified.resolution(&fixture.athlete), &CurrentAffiliationResolution::Unknown);
                let identities = dataset.verified_public_identities()?;
                check!(eq; identities.candidate_resolution(fixture.athlete.as_str()), Some(&census_domain::public_identity_history::PublicResolution::Unknown));
                check!(identities.public_id(fixture.athlete.as_str()).is_none());
                let selected = crate::report::Derivation::selected(
                    &dataset,
                    crate::report::Scope::AllSources,
                    None,
                    &[census_domain::UsJurisdiction::Wisconsin],
                    &verified,
                )?;
                check!(selected.athletes().is_empty());
                check!(selected
                    .affiliation_exclusions()
                    .get(&fixture.athlete)
                    .is_none());
            }
            check!(eq; serde_json::to_vec(&dataset)?, before);
        }
    }
    Ok(())
}

#[test]
fn explicit_opt_in_keeps_exact_profile_and_dataset_digest_guards() -> TestResult {
    let mut fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let exclude = fixture.profile.clone();
    let include = include_profile(&exclude)?;
    fixture.store.register_profile(&include)?;
    fixture.profile = include;
    let snapshot = fixture.store.snapshot();
    let mut dataset = fixture.dataset(&snapshot)?;
    validate_profile(&dataset, &fixture.profile)?;
    check!(validate_profile(&dataset, &exclude).is_err());
    let first = dataset.athletes.first_mut().ok_or("retained athlete")?;
    first.canonical_name.push_str(" changed");
    check!(validate_profile(&dataset, &fixture.profile).is_err());
    Ok(())
}

#[test]
fn explicit_opt_in_cannot_verify_a_missing_immutable_roster_body() -> TestResult {
    let mut fixture = Fixture::new("2026-10-07T12:00:00Z", "2026-10-07", true)?;
    let exclude = fixture.profile.clone();
    let include = include_profile(&exclude)?;
    fixture.store.register_profile(&include)?;
    std::fs::remove_file(&fixture.body_path)?;
    for profile in [exclude, include] {
        fixture.profile = profile;
        let snapshot = fixture.store.snapshot();
        let dataset = fixture.dataset(&snapshot)?;
        let verified = PinnedCurrentAffiliations::read(&dataset, &snapshot)?.into_verified();
        check!(eq; verified.resolution(&fixture.athlete), &CurrentAffiliationResolution::Unknown);
        identity::assert_unselected(&dataset, &verified, &fixture.athlete)?;
    }
    Ok(())
}
