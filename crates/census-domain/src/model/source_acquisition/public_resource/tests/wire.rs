use super::*;

#[test]
fn every_native_identifier_field_requires_canonical_positive_u64() -> TestResult {
    let maximum = u64::MAX.to_string();
    for (resource, _) in resources()? {
        let original = serde_json::to_value(resource)?;
        for field in [
            "athlete_id",
            "team_id",
            "meet_id",
            "result_id",
            "provider_school_id",
        ] {
            if original.get(field).is_none() {
                continue;
            }
            let mut accepted = original.clone();
            *accepted.get_mut(field).ok_or("fixture id field")? = serde_json::json!(maximum);
            let validated: PublicSourceResourceV1 = serde_json::from_value(accepted)?;
            check!(eq; serde_json::to_value(validated)?.get(field), Some(&serde_json::json!(maximum)));
            for invalid in [
                "",
                "0",
                "01",
                "+1",
                "-1",
                "1.0",
                "1e3",
                " 1",
                "1 ",
                "١",
                "18446744073709551616",
            ] {
                let mut changed = original.clone();
                *changed.get_mut(field).ok_or("fixture id field")? = serde_json::json!(invalid);
                check!(serde_json::from_value::<PublicSourceResourceV1>(changed).is_err());
            }
        }
    }
    for invalid in ["0", "01", "18446744073709551616"] {
        check!(PublicSourceResourceV1::milesplit_athlete_profile(invalid).is_err());
        check!(PublicSourceResourceV1::milesplit_team_roster(invalid).is_err());
        check!(PublicSourceResourceV1::milesplit_meet_team(invalid, "1").is_err());
        check!(PublicSourceResourceV1::milesplit_meet_team("1", invalid).is_err());
        check!(PublicSourceResourceV1::milesplit_raw_results("1", invalid).is_err());
        check!(PublicSourceResourceV1::milesplit_team_meet_history(invalid).is_err());
        check!(PublicSourceResourceV1::school_calendar(
            invalid,
            "https://www.example.org",
            Sport::IndoorTrack,
            year()?
        )
        .is_err());
    }
    Ok(())
}

#[test]
fn closed_wire_refuses_unknown_missing_duplicate_and_cross_variant_fields() -> TestResult {
    for (resource, _) in resources()? {
        let original = serde_json::to_value(&resource)?;
        let mut unknown = original.clone();
        unknown
            .as_object_mut()
            .ok_or("fixture object")?
            .insert("unsupported".into(), serde_json::json!(1));
        check!(serde_json::from_value::<PublicSourceResourceV1>(unknown).is_err());
        for field in original.as_object().ok_or("fixture object")?.keys() {
            let mut missing = original.clone();
            missing
                .as_object_mut()
                .ok_or("fixture object")?
                .remove(field);
            check!(serde_json::from_value::<PublicSourceResourceV1>(missing).is_err());
        }
        let encoded = serde_json::to_string(&resource)?;
        let duplicate = encoded.replacen('{', "{\"kind\":\"robots\",", 1);
        check!(serde_json::from_str::<PublicSourceResourceV1>(&duplicate).is_err());
    }
    for raw in [
        r#"{"kind":"unknown","team_id":"1"}"#,
        r#"{"kind":"mile_split_team_roster","team_id":"1","athlete_id":"2"}"#,
        r#"{"kind":"mile_split_team_roster","team_id":"1","team_id":"2"}"#,
        r#"{"kind":"robots","origin":"https://example.org","version":1}"#,
    ] {
        check!(serde_json::from_str::<PublicSourceResourceV1>(raw).is_err());
    }
    Ok(())
}

#[test]
fn school_calendar_preserves_known_sport_and_validated_school_year() -> TestResult {
    for sport in [Sport::CrossCountry, Sport::IndoorTrack, Sport::OutdoorTrack] {
        let resource = PublicSourceResourceV1::school_calendar(
            "12684",
            "https://www.example.org",
            sport,
            year()?,
        )?;
        let mut value = serde_json::to_value(&resource)?;
        check!(eq; serde_json::from_value::<PublicSourceResourceV1>(value.clone())?, resource);
        *value.get_mut("sport").ok_or("fixture sport")? = serde_json::json!("unknown");
        check!(serde_json::from_value::<PublicSourceResourceV1>(value).is_err());
        for invalid in [1899, 2101] {
            let mut value = serde_json::to_value(&resource)?;
            *value.get_mut("school_year").ok_or("fixture year")? = serde_json::json!(invalid);
            check!(serde_json::from_value::<PublicSourceResourceV1>(value).is_err());
        }
    }
    check!(PublicSourceResourceV1::school_calendar(
        "12684",
        "https://www.example.org",
        Sport::Unknown,
        year()?
    )
    .is_err());
    Ok(())
}
