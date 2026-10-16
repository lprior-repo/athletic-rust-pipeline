use super::*;

#[test]
fn capture_transport_duplicate_and_unknown_fields_are_rejected_directly() -> TestResult {
    let original = String::from_utf8(packet(DIRECTORY, ROSTER)?.encode()?)?;
    for (needle, replacement) in [
        ("\"directory\":{", "\"directory\":{\"status\":200,"),
        ("\"directory\":{", "\"directory\":{\"unsupported\":true,"),
        (
            "\"provenance\":{",
            "\"provenance\":{\"request_method\":\"GET\",",
        ),
        ("\"provenance\":{", "\"provenance\":{\"unsupported\":true,"),
    ] {
        let changed = original.replacen(needle, replacement, 1);
        check!(ne; changed, original);
        check!(serde_json::from_str::<ProviderSchoolPacketV1>(&changed).is_err());
    }
    Ok(())
}
