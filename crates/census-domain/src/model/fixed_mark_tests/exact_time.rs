use crate::model::{ExactSeconds, Mark, TimeError};

#[test]
fn time_order_is_integer_order() -> Result<(), Box<dyn std::error::Error>> {
    check!(ExactSeconds::parse("10.941")? < ExactSeconds::parse("10.944")?);
    check!(ExactSeconds::parse("10.94")? < ExactSeconds::parse("12.30")?);
    check!(ExactSeconds::parse("581.23")? < ExactSeconds::parse("595.11")?);
    Ok(())
}
#[test]
fn equivalent_time_spellings_share_equality_order_and_hash(
) -> Result<(), Box<dyn std::error::Error>> {
    let values = ["10.94", "10.940", "010.940000000"].map(ExactSeconds::parse);
    let mut hashes = std::collections::HashSet::new();
    let mut ordered = std::collections::BTreeSet::new();
    for value in values {
        let value = value?;
        hashes.insert(value);
        ordered.insert(value);
    }
    check!(eq; hashes.len(), 1);
    check!(eq; ordered.len(), 1);
    check!(eq; ExactSeconds::parse("10.940")?.to_string(), "10.940");
    Ok(())
}
#[test]
fn exact_seconds_preserve_nanosecond_boundaries_and_source_precision(
) -> Result<(), Box<dyn std::error::Error>> {
    for (raw, value, precision) in [
        ("0.000000001", 1, 9),
        ("10.941", 10_941_000_000, 3),
        ("60.00", 60_000_000_000, 2),
        ("9223372036.854775807", i64::MAX, 9),
    ] {
        let time = ExactSeconds::parse(raw)?;
        check!(eq; (time.value(), time.precision(), time.to_string()), (value, precision, raw.to_string()));
    }
    check!(eq; ExactSeconds::parse("9223372036.854775808"), Err(TimeError::Overflow));
    check!(eq; ExactSeconds::parse("9223372037"), Err(TimeError::Overflow));
    for raw in [
        "0",
        "-1",
        "NaN",
        "inf",
        "1e1",
        "1_0",
        "1.0000000000",
        "1.",
        ".1",
    ] {
        check!(eq; ExactSeconds::parse(raw), Err(TimeError::Invalid), "{raw}");
    }
    Ok(())
}
#[test]
fn exact_time_reader_refuses_legacy_scalar_without_explicit_migration(
) -> Result<(), Box<dyn std::error::Error>> {
    for raw in ["50.21", "5021", "null", "\"10.94\""] {
        check!(
            serde_json::from_str::<ExactSeconds>(raw).is_err(),
            "legacy scalar {raw} was accepted without migration"
        );
    }
    Ok(())
}
#[test]
fn exact_time_wire_retains_precision_and_rejects_inconsistent_parts(
) -> Result<(), Box<dyn std::error::Error>> {
    let time = ExactSeconds::parse("50.210")?;
    let json = serde_json::to_string(&time)?;
    check!(eq; json, r#"{"nanoseconds":50210000000,"precision":3}"#);
    let recovered: ExactSeconds = serde_json::from_str(&json)?;
    check!(eq; recovered.to_string(), "50.210");
    check!(eq; recovered.precision(), 3);
    check!(eq; ExactSeconds::from_parts(1, 8), Err(TimeError::Invalid));
    check!(eq; ExactSeconds::from_parts(1, 10), Err(TimeError::Invalid));
    check!(eq; ExactSeconds::from_parts(0, 9), Err(TimeError::Invalid));
    Ok(())
}
#[test]
fn mark_time_seconds_round_trips_explicit_nanoseconds() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"TimeSeconds":{"nanoseconds":50210000000,"precision":2}}"#;
    let mark: Mark = serde_json::from_str(json)?;
    check!(eq; mark, Mark::TimeSeconds(ExactSeconds::parse("50.21")?));
    check!(eq; serde_json::to_string(&mark)?, json);
    Ok(())
}
