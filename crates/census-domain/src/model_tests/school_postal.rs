use super::*;
use crate::school_directory::{PostalAddress, SourceLabel, StreetLine};

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
