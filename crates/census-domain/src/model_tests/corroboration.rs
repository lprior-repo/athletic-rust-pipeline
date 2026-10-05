use super::*;

fn milesplit(id: &str) -> SourceIdentity {
    SourceIdentity::new(SourceNamespace::MilesplitAthlete, id)
        .with_url(format!("https://milesplit.test/{id}"))
}

fn tfrrs(id: &str) -> SourceIdentity {
    SourceIdentity::new(SourceNamespace::TfrrsAthlete, id)
        .with_url(format!("https://tfrrs.test/{id}"))
}

fn observed(
    school: &str,
    source: SourceIdentity,
    links: &[SourceIdentity],
    document: &str,
) -> CanonicalAthlete {
    let school = SchoolId::mint("sch", &["WI", school]);
    let mut athlete = CanonicalAthlete::new(
        &school,
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Girls,
        source,
    );
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new("fixture", Some(document.to_owned())),
        "2026-09-26",
    ));
    for link in links {
        athlete.add_identity(link.clone());
    }
    athlete
}

fn attested(identity: &SourceIdentity, document: &str) -> (SourceIdentity, String) {
    (identity.clone(), document.to_owned())
}

fn pair(first: &CanonicalAthlete, second: &CanonicalAthlete) -> [AthleteCandidateId; 2] {
    [first.id.cast(), second.id.cast()]
}

#[test]
fn linked_object_corroborates_when_attested_independently() -> Result<(), Box<dyn std::error::Error>>
{
    let linked = observed(
        "School A",
        milesplit("111"),
        &[tfrrs("999")],
        "https://milesplit.test/111",
    );
    let owner = observed("School B", tfrrs("999"), &[], "https://tfrrs.test/999");
    let mut index = AthleteIdentityIndex::default();
    index.observe_attested(
        &linked,
        &[attested(
            &SourceIdentity::new(SourceNamespace::TfrrsAthlete, "999"),
            "https://milesplit.test/111",
        )],
    )?;
    index.observe(&owner)?;
    check!(index.supports_identity(AppliedIdentityKind::SamePerson, &pair(&linked, &owner)));
    Ok(())
}

#[test]
fn a_link_shared_pair_without_attestation_stays_review() -> Result<(), Box<dyn std::error::Error>> {
    let link = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "999");
    let linked = observed(
        "School A",
        milesplit("111"),
        std::slice::from_ref(&link),
        "https://milesplit.test/111",
    );
    let owner = observed("School B", tfrrs("999"), &[], "https://tfrrs.test/999");
    let mut index = AthleteIdentityIndex::default();
    index.observe(&linked)?;
    index.observe(&owner)?;
    check!(!index.supports_identity(AppliedIdentityKind::SamePerson, &pair(&linked, &owner)));
    Ok(())
}

#[test]
fn a_retained_link_document_corroborates_without_an_explicit_attestation(
) -> Result<(), Box<dyn std::error::Error>> {
    let linked = observed(
        "School A",
        milesplit("111"),
        &[tfrrs("999").with_url("https://milesplit.test/111")],
        "https://milesplit.test/111",
    );
    let owner = observed("School B", tfrrs("999"), &[], "https://tfrrs.test/999");
    let mut index = AthleteIdentityIndex::default();
    index.observe(&linked)?;
    index.observe(&owner)?;
    check!(index.supports_identity(AppliedIdentityKind::SamePerson, &pair(&linked, &owner)));
    Ok(())
}

#[test]
fn one_document_attesting_both_objects_does_not_corroborate(
) -> Result<(), Box<dyn std::error::Error>> {
    let linked = observed(
        "School A",
        milesplit("111"),
        &[tfrrs("999")],
        "https://milesplit.test/111",
    );
    let owner = observed("School B", tfrrs("999"), &[], "https://tfrrs.test/999");
    let mut index = AthleteIdentityIndex::default();
    index.observe(&linked)?;
    index.observe(&owner)?;
    check!(!index.supports_identity(AppliedIdentityKind::SamePerson, &pair(&linked, &owner)));
    Ok(())
}

#[test]
fn two_ids_from_one_provider_withhold() -> Result<(), Box<dyn std::error::Error>> {
    let first = observed(
        "School A",
        milesplit("111"),
        &[tfrrs("999")],
        "https://milesplit.test/111",
    );
    let second = observed(
        "School B",
        milesplit("222"),
        &[tfrrs("999")],
        "https://milesplit.test/222",
    );
    let owner = observed("School C", tfrrs("999"), &[], "https://tfrrs.test/999");
    let mut index = AthleteIdentityIndex::default();
    index.observe_attested(
        &first,
        &[attested(
            &SourceIdentity::new(SourceNamespace::TfrrsAthlete, "999"),
            "https://milesplit.test/111",
        )],
    )?;
    index.observe_attested(
        &second,
        &[attested(
            &SourceIdentity::new(SourceNamespace::TfrrsAthlete, "999"),
            "https://milesplit.test/222",
        )],
    )?;
    index.observe(&owner)?;
    check!(!index.supports_identity(
        AppliedIdentityKind::SamePerson,
        &[first.id.cast(), second.id.cast(), owner.id.cast()],
    ));
    Ok(())
}

#[test]
fn same_document_attestation_does_not_corroborate() -> Result<(), Box<dyn std::error::Error>> {
    let link = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "999");
    let linked = observed(
        "School A",
        milesplit("111"),
        std::slice::from_ref(&link),
        "https://milesplit.test/111",
    );
    let owner = observed(
        "School B",
        link.clone(),
        &[],
        "https://shared.test/one-page",
    );
    let mut index = AthleteIdentityIndex::default();
    index.observe_attested(&linked, &[attested(&link, "https://shared.test/one-page")])?;
    index.observe(&owner)?;
    check!(!index.supports_identity(AppliedIdentityKind::SamePerson, &pair(&linked, &owner)));
    Ok(())
}

#[test]
fn an_unparsed_member_withholds_corroboration() -> Result<(), Box<dyn std::error::Error>> {
    let linked = observed(
        "School A",
        milesplit("111"),
        &[tfrrs("999")],
        "https://milesplit.test/111",
    );
    let mut owner = observed("School B", tfrrs("999"), &[], "https://tfrrs.test/999");
    owner.evidence[0].method = EvidenceMethod::Fetched;
    let mut index = AthleteIdentityIndex::default();
    index.observe_attested(
        &linked,
        &[attested(
            &SourceIdentity::new(SourceNamespace::TfrrsAthlete, "999"),
            "https://milesplit.test/111",
        )],
    )?;
    index.observe(&owner)?;
    check!(!index.supports_identity(AppliedIdentityKind::SamePerson, &pair(&linked, &owner)));
    Ok(())
}

#[test]
fn a_contradictory_third_member_withholds() -> Result<(), Box<dyn std::error::Error>> {
    let linked = observed(
        "School A",
        milesplit("111"),
        &[tfrrs("999")],
        "https://milesplit.test/111",
    );
    let owner = observed("School B", tfrrs("999"), &[], "https://tfrrs.test/999");
    let mut third = observed("School C", tfrrs("777"), &[], "https://tfrrs.test/777");
    third.gender = Gender::Boys;
    let mut index = AthleteIdentityIndex::default();
    index.observe_attested(
        &linked,
        &[attested(
            &SourceIdentity::new(SourceNamespace::TfrrsAthlete, "999"),
            "https://milesplit.test/111",
        )],
    )?;
    index.observe(&owner)?;
    index.observe(&third)?;
    check!(index.supports_identity(AppliedIdentityKind::SamePerson, &pair(&linked, &owner)));
    check!(!index.supports_identity(
        AppliedIdentityKind::SamePerson,
        &[linked.id.cast(), owner.id.cast(), third.id.cast()],
    ));
    Ok(())
}

#[test]
fn name_school_and_cohort_agreement_alone_is_not_positive_evidence(
) -> Result<(), Box<dyn std::error::Error>> {
    let first = observed(
        "School A",
        milesplit("111"),
        &[],
        "https://milesplit.test/111",
    );
    let second = observed(
        "School A",
        milesplit("222"),
        &[],
        "https://milesplit.test/222",
    );
    let mut index = AthleteIdentityIndex::default();
    index.observe(&first)?;
    index.observe(&second)?;
    check!(!index.supports_identity(AppliedIdentityKind::SamePerson, &pair(&first, &second)));
    Ok(())
}
