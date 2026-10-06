use super::*;
use crate::model::SourceNamespace;
use crate::school_directory::StreetLine;
use crate::UsJurisdiction;

fn owner(id: &str) -> SourceIdentity {
    SourceIdentity::new(SourceNamespace::association_school("wiaa"), id)
}

fn claim(
    street: StreetLine,
    id: &str,
    url: &str,
    day: &str,
    label: SourceLabel,
) -> Result<SchoolPostalAddress, SchoolAddressError> {
    SchoolPostalAddress::new(
        PostalAddress::line(street),
        owner(id),
        label,
        Evidence::parsed(super::super::SourceRef::new("wiaa", Some(url.into())), day),
        "a".repeat(64),
    )
}

fn source_label() -> SourceLabel {
    SourceLabel::AthleticAssociation {
        state: UsJurisdiction::Wisconsin,
    }
}

#[test]
fn malformed_postal_capture_dates_and_nonhttp_locators_cannot_enter_publication(
) -> Result<(), Box<dyn std::error::Error>> {
    let street = StreetLine::parse("1 Rocket Drive")?;
    for (url, day) in [
        ("not-a-url", "2026-09-27"),
        ("file:///tmp/private", "2026-09-27"),
        ("https://", "2026-09-27"),
        ("https://user:secret@schools.test/capture", "2026-09-27"),
        ("https://schools.test/capture", "not-a-date"),
        ("https://schools.test/capture", "2026-02-30"),
        ("https://schools.test/capture", "2026-9-7"),
        ("https://schools.test/capture", "2026-02-30T12:34:56Z"),
        ("https://schools.test/capture", "2026-10-01T25:34:56Z"),
        ("https://schools.test/capture", "2026-10-01T12:34:56"),
    ] {
        check!(claim(street.clone(), "1001", url, day, source_label()).is_err());
    }
    let valid = claim(
        street.clone(),
        "1001",
        "https://schools.test/capture",
        "2024-02-29",
        source_label(),
    )?;
    let encoded = serde_json::to_value(&valid)?;
    check!(eq; serde_json::from_value::<SchoolPostalAddress>(encoded.clone())?,
    valid);
    let mut invalid = encoded;
    invalid["evidence"]["observed_on"] = serde_json::json!("2023-02-29");
    check!(serde_json::from_value::<SchoolPostalAddress>(invalid).is_err());
    let timestamp = "2026-10-01T12:34:56Z";
    let observed = claim(
        street,
        "1001",
        "https://schools.test/capture",
        timestamp,
        source_label(),
    )?;
    check!(eq; observed.evidence().observed_on, timestamp);
    Ok(())
}

#[test]
fn metadata_owner_line_breaks_cannot_forge_aligned_postal_rows(
) -> Result<(), Box<dyn std::error::Error>> {
    let street = StreetLine::parse("1 Rocket Drive")?;
    for id in [
        "1001\nforeign",
        "1001\rforeign",
        "1001\tforeign",
        "1001 foreign",
        "1001\0foreign",
    ] {
        check!(eq; claim(
            street.clone(),
            id,
            "https://schools.test/capture",
            "2026-09-27",
            source_label()
        ),
        Err(SchoolAddressError::MissingOwner));
    }
    Ok(())
}

#[test]
fn source_jurisdiction_binds_the_school_even_when_address_state_is_missing(
) -> Result<(), Box<dyn std::error::Error>> {
    let street = StreetLine::parse("1 Rocket Drive")?;
    let mut school = CanonicalSchool::new(
        UsJurisdiction::NorthCarolina,
        "Rocket High",
        "rocket-high",
        None,
    )
    .0;
    school.source_identities.push(owner("1001"));
    let foreign = claim(
        street,
        "1001",
        "https://schools.test/capture",
        "2026-09-27",
        source_label(),
    )?;
    check!(eq; school.add_postal_address(foreign),
    Err(SchoolAddressError::ForeignJurisdiction));
    check!(school.postal_addresses.is_empty());
    Ok(())
}

#[test]
fn unrelated_source_labels_cannot_relabel_an_association_owned_capture(
) -> Result<(), Box<dyn std::error::Error>> {
    let street = StreetLine::parse("1 Rocket Drive")?;
    for label in [SourceLabel::Ccd, SourceLabel::Pss, SourceLabel::Geocoder] {
        check!(claim(
            street.clone(),
            "1001",
            "https://schools.test/capture",
            "2026-09-27",
            label
        )
        .is_err());
    }
    Ok(())
}
