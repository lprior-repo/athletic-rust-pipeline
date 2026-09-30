use crate::school_directory::{
    CityName, Coordinates, Latitude, PostalAddress, StreetLine, ZipCode,
};

#[test]
fn zip_code_accepts_the_three_published_shapes() {
    let plain = ZipCode::parse("35950").expect("five digits parse");
    assert_eq!(plain.code(), "35950");
    assert_eq!(plain.plus4(), None);
    assert_eq!(plain.to_string(), "35950");

    let hyphenated = ZipCode::parse("35950-2336").expect("hyphenated zip parses");
    assert_eq!(hyphenated.plus4(), Some("2336"));
    assert_eq!(hyphenated.to_string(), "35950-2336");

    let conjoined = ZipCode::parse("359502336").expect("nine digits parse");
    assert_eq!(conjoined, hyphenated);

    let columns = ZipCode::of("35950", Some("2336")).expect("columns parse");
    assert_eq!(columns, hyphenated);

    let empty_extension = ZipCode::of("35950", Some("  ")).expect("blank extension is absent");
    assert_eq!(empty_extension, plain);

    let padded = ZipCode::parse(" 35950 ").expect("surrounding space is trimmed");
    assert_eq!(padded, plain);
}

#[test]
fn zip_code_refuses_every_other_shape() {
    for raw in [
        "",
        "3595",
        "35950-12",
        "35950-23361",
        "35950-2336-1",
        "ABCDE",
        "3595O",
    ] {
        assert!(ZipCode::parse(raw).is_err(), "{raw:?} must not parse");
    }
    assert!(ZipCode::of("3595A", None).is_err());
    assert!(ZipCode::of("35950", Some("233X")).is_err());
}

#[test]
fn mixed_case_input_is_left_alone_and_all_caps_is_presented() {
    let mixed = StreetLine::parse("600 E Alabama Ave").expect("mixed case parses");
    assert_eq!(mixed.as_str(), "600 E Alabama Ave");

    let shouting = StreetLine::parse("600 EAST ALABAMA AVENUE").expect("caps parse");
    assert_eq!(shouting.as_str(), "600 East Alabama Avenue");

    let prose = CityName::parse("SAINT MARYS OF THE LAKE").expect("caps city parses");
    assert_eq!(prose.as_str(), "Saint Marys of the Lake");

    let particle_first = CityName::parse("LA PORTE").expect("caps city parses");
    assert_eq!(particle_first.as_str(), "La Porte");

    let hyphenated = CityName::parse("WINSTON-SALEM").expect("caps city parses");
    assert_eq!(hyphenated.as_str(), "Winston-Salem");

    let possessive = StreetLine::parse("ST. MARY'S ROAD").expect("caps street parses");
    assert_eq!(possessive.as_str(), "St. Mary's Road");
}

#[test]
fn street_and_city_hygiene_collapses_whitespace_and_refuses_junk() {
    let collapsed = StreetLine::parse("  600   E \t Alabama  Ave ").expect("spacing parses");
    assert_eq!(collapsed.as_str(), "600 E Alabama Ave");

    assert!(StreetLine::parse("   ").is_err());
    assert!(CityName::parse("\u{0}").is_err());
    assert!(CityName::parse("Spring\u{7}field").is_err());

    let long = "a".repeat(121);
    assert!(StreetLine::parse(&long).is_err());
    let limit = "a".repeat(120);
    assert!(StreetLine::parse(&limit).is_ok());
}

#[test]
fn a_postal_address_carries_only_the_parts_it_has() {
    let street = StreetLine::parse("600 E Alabama Ave").expect("street parses");
    let city = CityName::parse("Albertville").expect("city parses");
    let zip = ZipCode::parse("35950").expect("zip parses");

    let bare = PostalAddress::line(street.clone());
    assert_eq!(bare.line1(), Some(&street));
    assert!(bare.city().is_none());
    assert!(bare.state().is_none());
    assert!(bare.zip().is_none());

    let full = bare
        .clone()
        .with_city(city.clone())
        .with_state(crate::UsJurisdiction::Alabama)
        .with_zip(zip.clone());
    assert_eq!(full.city(), Some(&city));
    assert_eq!(full.state(), Some(crate::UsJurisdiction::Alabama));
    assert_eq!(full.zip(), Some(&zip));
}

#[test]
fn coordinates_are_exact_scaled_integers() {
    let coordinates = Coordinates::parse("33.46998", "-86.924519").expect("google text parses");
    assert_eq!(coordinates.latitude().scaled(), 334_699_800);
    assert_eq!(coordinates.longitude().scaled(), -869_245_190);
    assert_eq!(coordinates.latitude().to_string(), "33.46998");
    assert_eq!(coordinates.longitude().to_string(), "-86.924519");

    let whole = Coordinates::parse("45", "-122").expect("whole degrees parse");
    assert_eq!(whole.latitude().to_string(), "45");
    assert_eq!(whole.longitude().to_string(), "-122");
}

#[test]
fn coordinates_round_the_eighth_digit_and_refuse_out_of_range() {
    let rounded = Latitude::from_decimal_text("1.00000005").expect("rounding parses");
    assert_eq!(rounded.to_string(), "1.0000001");
    let truncated = Latitude::from_decimal_text("1.000000049").expect("extra digits parse");
    assert_eq!(truncated.to_string(), "1");

    assert!(Latitude::from_decimal_text("90.0000001").is_err());
    assert!(Latitude::from_decimal_text("90").is_ok());
    assert!(Latitude::from_decimal_text("91").is_err());
    assert!(crate::school_directory::Longitude::from_decimal_text("180").is_ok());
    assert!(crate::school_directory::Longitude::from_decimal_text("-180.5").is_err());
    for raw in ["", "north", "1.2.3", "--1", "1e5", " 1 2 "] {
        assert!(
            Latitude::from_decimal_text(raw).is_err(),
            "{raw:?} must not parse"
        );
    }
}
