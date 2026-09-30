use crate::school_directory::{Coordinates, DirectoryError, Latitude, Longitude};

#[test]
fn a_coordinate_longer_than_the_limit_is_refused_before_it_is_scanned() {
    let overlong = format!("1.{}", "9".repeat(4_096));
    let error =
        Latitude::from_decimal_text(&overlong).expect_err("an overlong latitude is refused");
    assert!(matches!(
        error,
        DirectoryError::FieldTooLong {
            field: "latitude",
            limit: 32,
            ..
        }
    ));
    assert!(Latitude::from_decimal_text("1.2345678").is_ok());
    let at_limit = format!("-1.{}", "0".repeat(28));
    assert_eq!(at_limit.len(), 31);
    assert!(Longitude::from_decimal_text(&at_limit).is_ok());
}

#[test]
fn coordinates_still_round_the_ninth_fraction_digit() {
    let rounded = Coordinates::parse("33.46998254", "-86.92451949").expect("coordinates parse");
    assert_eq!(rounded.latitude().to_string(), "33.4699825");
    assert_eq!(rounded.longitude().to_string(), "-86.9245195");
}
