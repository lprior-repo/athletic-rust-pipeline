use super::*;
use crate::school_directory::{CityName, PostalAddress, SourceLabel, StreetLine, ZipCode};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn components() -> TestResult<(PostalAddress, SourceIdentity, SourceLabel, Evidence, String)> {
    Ok((
        PostalAddress::line(StreetLine::parse("1 Source Road")?),
        SourceIdentity::new(SourceNamespace::association_school("nchsaa"), "ZCUM49"),
        SourceLabel::AthleticAssociation {
            state: UsJurisdiction::NorthCarolina,
        },
        Evidence::parsed(
            SourceRef::new(
                "nchsaa",
                Some("https://source.test/schools/ZCUM49/summary".into()),
            ),
            "2026-09-27",
        ),
        "a".repeat(64),
    ))
}

fn assert_rejected(
    parts: (PostalAddress, SourceIdentity, SourceLabel, Evidence, String),
    expected: SchoolAddressError,
) -> TestResult {
    let (address, owner, source_label, evidence, capture_sha256) = parts;
    let wire = serde_json::json!({
        "address": address, "owner": owner, "source_label": source_label,
        "evidence": evidence, "capture_sha256": capture_sha256,
    });
    check!(eq; SchoolPostalAddress::new(address, owner, source_label, evidence, capture_sha256),
    Err(expected));
    let error = serde_json::from_value::<SchoolPostalAddress>(wire)
        .err()
        .ok_or("deserialization accepted invalid postal provenance")?;
    check!(error.to_string().contains(&expected.to_string()), "{error}");
    Ok(())
}

#[test]
fn postal_provenance_cannot_inject_extra_claim_positions_or_exceed_capture_budgets() -> TestResult {
    for id in ["ZCUM49\nforged".to_string(), "x".repeat(257)] {
        let mut parts = components()?;
        parts.1.id = id;
        assert_rejected(parts, SchoolAddressError::MissingOwner)?;
    }
    let mut parts = components()?;
    parts.1.namespace = SourceNamespace::association_school("nchsaa\nforged");
    assert_rejected(parts, SchoolAddressError::MissingOwner)?;
    for note in ["first\nsecond".to_string(), "x".repeat(4097)] {
        let mut parts = components()?;
        parts.3.note = Some(note);
        assert_rejected(parts, SchoolAddressError::InvalidProvenance)?;
    }
    for date in ["2026-09-27\n2026-09-28", "2026-99-99", "not-a-date"] {
        let mut parts = components()?;
        parts.3.observed_on = date.into();
        assert_rejected(parts, SchoolAddressError::InvalidObservationDate)?;
    }
    for url in [
        "https://source.test/one\nhttps://forged.test/two".into(),
        "https://user:secret@source.test/school".into(),
        "https://".into(),
        format!("https://source.test/{}", "x".repeat(2048)),
    ] {
        let mut parts = components()?;
        parts.3.source.url = Some(url);
        assert_rejected(parts, SchoolAddressError::InvalidSourceUrl)?;
    }
    Ok(())
}

#[test]
fn unsupported_or_foreign_source_authority_cannot_be_relabelled_as_a_postal_claim() -> TestResult {
    for label in [SourceLabel::Ccd, SourceLabel::Pss, SourceLabel::Geocoder] {
        let mut parts = components()?;
        parts.2 = label;
        assert_rejected(parts, SchoolAddressError::UnsupportedAuthority)?;
    }
    let mut parts = components()?;
    parts.3.source.id = "foreign-provider".into();
    assert_rejected(parts, SchoolAddressError::SourceAuthorityMismatch)?;
    Ok(())
}

#[test]
fn street_only_claim_still_cannot_attach_across_source_jurisdictions() -> TestResult {
    let (address, owner, _, mut evidence, hash) = components()?;
    evidence.source.id = "wiaa".into();
    let owner = SourceIdentity::new(SourceNamespace::association_school("wiaa"), owner.id);
    let claim = SchoolPostalAddress::new(
        address,
        owner.clone(),
        SourceLabel::AthleticAssociation {
            state: UsJurisdiction::Wisconsin,
        },
        evidence,
        hash,
    )?;
    let mut school = CanonicalSchool::new(
        UsJurisdiction::NorthCarolina,
        "Source School",
        "source school",
    )
    .0;
    school.source_identities.push(owner);
    check!(eq; school.add_postal_address(claim),
    Err(SchoolAddressError::ForeignJurisdiction));
    check!(eq; school.postal_addresses, Vec::new());
    Ok(())
}

#[test]
fn complete_postal_claim_order_is_independent_of_arrival_and_deserialized_vector_order(
) -> TestResult {
    let (address, owner, label, evidence, hash) = components()?;
    let first = SchoolPostalAddress::new(
        address,
        owner.clone(),
        label.clone(),
        evidence.clone(),
        hash,
    )?;
    let second = SchoolPostalAddress::new(
        PostalAddress::line(StreetLine::parse("2 Disputed Road")?),
        owner.clone(),
        label,
        Evidence {
            observed_on: "2026-09-28".into(),
            ..evidence
        },
        "b".repeat(64),
    )?;
    let mut forward = CanonicalSchool::new(
        UsJurisdiction::NorthCarolina,
        "Source School",
        "source school",
    )
    .0;
    forward.source_identities.push(owner);
    let mut reverse = forward.clone();
    forward.add_postal_address(first.clone())?;
    forward.add_postal_address(second.clone())?;
    reverse.add_postal_address(second.clone())?;
    reverse.add_postal_address(first.clone())?;
    reverse.add_postal_address(first.clone())?;
    check!(eq; forward.postal_addresses,
    vec![first.clone(), second.clone()]);
    check!(eq; reverse.postal_addresses, forward.postal_addresses);
    let mut wire = serde_json::to_value(&forward)?;
    wire["postal_addresses"] = serde_json::json!([second, first.clone(), first]);
    let reopened: CanonicalSchool = serde_json::from_value(wire)?;
    check!(eq; reopened.postal_addresses, forward.postal_addresses);
    Ok(())
}

fn directory_components(
) -> TestResult<(PostalAddress, SourceIdentity, SourceLabel, Evidence, String)> {
    Ok((
        PostalAddress::line(StreetLine::parse("100 Directory Way")?)
            .with_city(CityName::parse("Springfield")?)
            .with_state(UsJurisdiction::Ohio)
            .with_zip(ZipCode::parse("45501")?),
        SourceIdentity::new(
            SourceNamespace::school_directory("nces-ccd", UsJurisdiction::Ohio),
            "390000000001",
        ),
        SourceLabel::Ccd,
        Evidence::parsed(
            SourceRef::new(
                "nces-ccd",
                Some("https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip".into()),
            ),
            "2026-10-04",
        ),
        "d1473136285b5994b73a1a8b640757811eb81e0ae770953bcf915ee8c422386e".into(),
    ))
}

#[test]
fn directory_provider_claims_attach_only_through_their_own_namespace() -> TestResult {
    let (address, owner, label, evidence, hash) = directory_components()?;
    let claim = SchoolPostalAddress::new(address, owner.clone(), label, evidence, hash)?;
    let mut school = CanonicalSchool::new(
        UsJurisdiction::Ohio,
        "Springfield High School",
        "springfield high school",
    )
    .0;
    check!(eq; school.add_postal_address(claim.clone()),
    Err(SchoolAddressError::ForeignOwner));
    school.source_identities.push(owner);
    check!(eq; school.add_postal_address(claim), Ok(()));
    check!(eq; school.postal_addresses.len(), 1);
    Ok(())
}

#[test]
fn directory_claims_require_a_matching_provider_and_label() -> TestResult {
    let (address, owner, label, evidence, hash) = directory_components()?;
    let mut mismatched = owner.clone();
    mismatched.namespace = SourceNamespace::school_directory("nces-pss", UsJurisdiction::Ohio);
    assert_rejected(
        (
            address.clone(),
            mismatched,
            SourceLabel::Ccd,
            evidence.clone(),
            hash.clone(),
        ),
        SchoolAddressError::UnsupportedAuthority,
    )?;
    assert_rejected(
        (
            address.clone(),
            owner.clone(),
            SourceLabel::AthleticAssociation {
                state: UsJurisdiction::Ohio,
            },
            evidence.clone(),
            hash.clone(),
        ),
        SchoolAddressError::UnsupportedAuthority,
    )?;
    let mut foreign_evidence = evidence.clone();
    foreign_evidence.source.id = "nces-pss".into();
    assert_rejected(
        (
            address.clone(),
            owner.clone(),
            SourceLabel::Ccd,
            foreign_evidence,
            hash.clone(),
        ),
        SchoolAddressError::SourceAuthorityMismatch,
    )?;
    let claim = SchoolPostalAddress::new(address, owner, label, evidence, hash)?;
    let mut school = CanonicalSchool::new(
        UsJurisdiction::Indiana,
        "Springfield High School",
        "springfield high school",
    )
    .0;
    school.source_identities.push(claim.owner().clone());
    check!(eq; school.add_postal_address(claim),
    Err(SchoolAddressError::ForeignJurisdiction));
    Ok(())
}

#[test]
fn directory_claims_survive_wire_round_trips_and_reject_tampered_providers() -> TestResult {
    let (address, owner, label, evidence, hash) = directory_components()?;
    let claim = SchoolPostalAddress::new(address, owner, label, evidence, hash)?;
    let encoded = serde_json::to_value(&claim)?;
    check!(eq; serde_json::from_value::<SchoolPostalAddress>(encoded.clone())?, claim);
    let mut tampered = encoded;
    tampered["source_label"] = serde_json::json!("pss");
    check!(serde_json::from_value::<SchoolPostalAddress>(tampered).is_err());
    Ok(())
}
