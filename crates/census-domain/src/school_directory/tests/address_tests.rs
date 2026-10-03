use super::TestResult;
use crate::school_directory::{
    CityName, Coordinates, Latitude, PostalAddress, StreetLine, ZipCode,
};

#[test]
fn zip_code_accepts_the_three_published_shapes() -> TestResult {
    let plain = ZipCode::parse("35950")?;
    check!(eq; plain.code(), "35950");
    check!(eq; plain.plus4(), None);
    check!(eq; plain.to_string(), "35950");

    let hyphenated = ZipCode::parse("35950-2336")?;
    check!(eq; hyphenated.plus4(), Some("2336"));
    check!(eq; hyphenated.to_string(), "35950-2336");

    let conjoined = ZipCode::parse("359502336")?;
    check!(eq; conjoined, hyphenated);

    let columns = ZipCode::of("35950", Some("2336"))?;
    check!(eq; columns, hyphenated);

    let empty_extension = ZipCode::of("35950", Some("  "))?;
    check!(eq; empty_extension, plain);

    let padded = ZipCode::parse(" 35950 ")?;
    check!(eq; padded, plain);
    Ok(())
}

#[test]
fn zip_code_refuses_every_other_shape() -> Result<(), Box<dyn std::error::Error>> {
    for raw in [
        "",
        "3595",
        "35950-12",
        "35950-23361",
        "35950-2336-1",
        "ABCDE",
        "3595O",
    ] {
        check!(ZipCode::parse(raw).is_err(), "{raw:?} must not parse");
    }
    check!(ZipCode::of("3595A", None).is_err());
    check!(ZipCode::of("35950", Some("233X")).is_err());
    Ok(())
}

#[test]
fn mixed_case_input_is_left_alone_and_all_caps_is_presented() -> TestResult {
    let mixed = StreetLine::parse("600 E Alabama Ave")?;
    check!(eq; mixed.as_str(), "600 E Alabama Ave");

    let shouting = StreetLine::parse("600 EAST ALABAMA AVENUE")?;
    check!(eq; shouting.as_str(), "600 East Alabama Avenue");

    let prose = CityName::parse("SAINT MARYS OF THE LAKE")?;
    check!(eq; prose.as_str(), "Saint Marys of the Lake");

    let particle_first = CityName::parse("LA PORTE")?;
    check!(eq; particle_first.as_str(), "La Porte");

    let hyphenated = CityName::parse("WINSTON-SALEM")?;
    check!(eq; hyphenated.as_str(), "Winston-Salem");

    let possessive = StreetLine::parse("ST. MARY'S ROAD")?;
    check!(eq; possessive.as_str(), "St. Mary's Road");
    Ok(())
}

#[test]
fn street_and_city_hygiene_collapses_whitespace_and_refuses_junk() -> TestResult {
    let collapsed = StreetLine::parse("  600   E \t Alabama  Ave ")?;
    check!(eq; collapsed.as_str(), "600 E Alabama Ave");

    check!(StreetLine::parse("   ").is_err());
    check!(CityName::parse("\u{0}").is_err());
    check!(CityName::parse("Spring\u{7}field").is_err());

    let long = "a".repeat(121);
    check!(StreetLine::parse(&long).is_err());
    let limit = "a".repeat(120);
    check!(StreetLine::parse(&limit).is_ok());
    Ok(())
}

#[test]
fn a_postal_address_carries_only_the_parts_it_has() -> TestResult {
    let street = StreetLine::parse("600 E Alabama Ave")?;
    let city = CityName::parse("Albertville")?;
    let zip = ZipCode::parse("35950")?;

    let bare = PostalAddress::line(street.clone());
    check!(eq; bare.line1(), Some(&street));
    check!(bare.city().is_none());
    check!(bare.state().is_none());
    check!(bare.zip().is_none());

    let full = bare
        .clone()
        .with_city(city.clone())
        .with_state(crate::UsJurisdiction::Alabama)
        .with_zip(zip.clone());
    check!(eq; full.city(), Some(&city));
    check!(eq; full.state(), Some(crate::UsJurisdiction::Alabama));
    check!(eq; full.zip(), Some(&zip));
    Ok(())
}

#[test]
fn coordinates_are_exact_scaled_integers() -> TestResult {
    let coordinates = Coordinates::parse("33.46998", "-86.924519")?;
    check!(eq; coordinates.latitude().scaled(), 334_699_800);
    check!(eq; coordinates.longitude().scaled(), -869_245_190);
    check!(eq; coordinates.latitude().to_string(), "33.46998");
    check!(eq; coordinates.longitude().to_string(), "-86.924519");

    let whole = Coordinates::parse("45", "-122")?;
    check!(eq; whole.latitude().to_string(), "45");
    check!(eq; whole.longitude().to_string(), "-122");
    Ok(())
}

#[test]
fn coordinates_round_the_eighth_digit_and_refuse_out_of_range() -> TestResult {
    let rounded = Latitude::from_decimal_text("1.00000005")?;
    check!(eq; rounded.to_string(), "1.0000001");
    let truncated = Latitude::from_decimal_text("1.000000049")?;
    check!(eq; truncated.to_string(), "1");

    check!(Latitude::from_decimal_text("90.0000001").is_err());
    check!(Latitude::from_decimal_text("90").is_ok());
    check!(Latitude::from_decimal_text("91").is_err());
    check!(crate::school_directory::Longitude::from_decimal_text("180").is_ok());
    check!(crate::school_directory::Longitude::from_decimal_text("-180.5").is_err());
    for raw in ["", "north", "1.2.3", "--1", "1e5", " 1 2 "] {
        check!(
            Latitude::from_decimal_text(raw).is_err(),
            "{raw:?} must not parse"
        );
    }
    Ok(())
}
