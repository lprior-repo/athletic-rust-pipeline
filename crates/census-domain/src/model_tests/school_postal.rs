use super::*;
use crate::school_directory::{PostalAddress, SourceLabel, StreetLine};

fn components() -> (PostalAddress, SourceIdentity, SourceLabel, Evidence, String) {
    (
        PostalAddress::line(StreetLine::parse("1 Source Road").unwrap()),
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
    )
}

fn assert_rejected(
    parts: (PostalAddress, SourceIdentity, SourceLabel, Evidence, String),
    expected: SchoolAddressError,
) {
    let (address, owner, source_label, evidence, capture_sha256) = parts;
    let wire = serde_json::json!({
        "address": address, "owner": owner, "source_label": source_label,
        "evidence": evidence, "capture_sha256": capture_sha256,
    });
    assert_eq!(
        SchoolPostalAddress::new(address, owner, source_label, evidence, capture_sha256),
        Err(expected)
    );
    let error = serde_json::from_value::<SchoolPostalAddress>(wire).unwrap_err();
    assert!(error.to_string().contains(&expected.to_string()), "{error}");
}

#[test]
fn postal_provenance_cannot_inject_extra_claim_positions_or_exceed_capture_budgets() {
    for id in ["ZCUM49\nforged".to_string(), "x".repeat(257)] {
        let mut parts = components();
        parts.1.id = id;
        assert_rejected(parts, SchoolAddressError::MissingOwner);
    }
    let mut parts = components();
    parts.1.namespace = SourceNamespace::association_school("nchsaa\nforged");
    assert_rejected(parts, SchoolAddressError::MissingOwner);
    for note in ["first\nsecond".to_string(), "x".repeat(4097)] {
        let mut parts = components();
        parts.3.note = Some(note);
        assert_rejected(parts, SchoolAddressError::InvalidProvenance);
    }
    for date in ["2026-09-27\n2026-09-28", "2026-99-99", "not-a-date"] {
        let mut parts = components();
        parts.3.observed_on = date.into();
        assert_rejected(parts, SchoolAddressError::InvalidObservationDate);
    }
    for url in [
        "https://source.test/one\nhttps://forged.test/two".into(),
        "https://user:secret@source.test/school".into(),
        "https://".into(),
        format!("https://source.test/{}", "x".repeat(2048)),
    ] {
        let mut parts = components();
        parts.3.source.url = Some(url);
        assert_rejected(parts, SchoolAddressError::InvalidSourceUrl);
    }
}

#[test]
fn unsupported_or_foreign_source_authority_cannot_be_relabelled_as_a_postal_claim() {
    for label in [SourceLabel::Ccd, SourceLabel::Pss, SourceLabel::Geocoder] {
        let mut parts = components();
        parts.2 = label;
        assert_rejected(parts, SchoolAddressError::UnsupportedAuthority);
    }
    let mut parts = components();
    parts.3.source.id = "foreign-provider".into();
    assert_rejected(parts, SchoolAddressError::SourceAuthorityMismatch);
}

#[test]
fn street_only_claim_still_cannot_attach_across_source_jurisdictions() {
    let (address, owner, _, mut evidence, hash) = components();
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
    )
    .unwrap();
    let mut school = CanonicalSchool::new(
        UsJurisdiction::NorthCarolina,
        "Source School",
        "source school",
    )
    .0;
    school.source_identities.push(owner);
    assert_eq!(
        school.add_postal_address(claim),
        Err(SchoolAddressError::ForeignJurisdiction)
    );
    assert_eq!(school.postal_addresses, Vec::new());
}

#[test]
fn complete_postal_claim_order_is_independent_of_arrival_and_deserialized_vector_order() {
    let (address, owner, label, evidence, hash) = components();
    let first = SchoolPostalAddress::new(
        address,
        owner.clone(),
        label.clone(),
        evidence.clone(),
        hash,
    )
    .unwrap();
    let second = SchoolPostalAddress::new(
        PostalAddress::line(StreetLine::parse("2 Disputed Road").unwrap()),
        owner.clone(),
        label,
        Evidence {
            observed_on: "2026-09-28".into(),
            ..evidence
        },
        "b".repeat(64),
    )
    .unwrap();
    let mut forward = CanonicalSchool::new(
        UsJurisdiction::NorthCarolina,
        "Source School",
        "source school",
    )
    .0;
    forward.source_identities.push(owner);
    let mut reverse = forward.clone();
    forward.add_postal_address(first.clone()).unwrap();
    forward.add_postal_address(second.clone()).unwrap();
    reverse.add_postal_address(second.clone()).unwrap();
    reverse.add_postal_address(first.clone()).unwrap();
    reverse.add_postal_address(first.clone()).unwrap();
    assert_eq!(
        forward.postal_addresses,
        vec![first.clone(), second.clone()]
    );
    assert_eq!(reverse.postal_addresses, forward.postal_addresses);
    let mut wire = serde_json::to_value(&forward).unwrap();
    wire["postal_addresses"] = serde_json::json!([second, first.clone(), first]);
    let reopened: CanonicalSchool = serde_json::from_value(wire).unwrap();
    assert_eq!(reopened.postal_addresses, forward.postal_addresses);
}
