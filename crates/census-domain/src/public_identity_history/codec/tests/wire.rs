use super::*;
use fixture::*;

#[test]
fn existing_version_one_bytes_roundtrip_for_all_six_request_variants() -> TestResult {
    roundtrip(&complete()?)
}

#[test]
fn opaque_candidate_ids_are_preserved_without_an_invented_prefix() -> TestResult {
    roundtrip(&Fixture::one()?)
}

#[test]
fn empty_history_requires_exact_genesis() -> TestResult {
    roundtrip(&Fixture::new()?)
}

#[test]
fn supported_athlete_namespaces_keep_their_existing_wire_shapes() -> TestResult {
    for namespace in [
        SourceNamespace::TfrrsAthlete,
        SourceNamespace::MilesplitAthlete,
        SourceNamespace::DirectAthleticsAthlete,
        SourceNamespace::AssociationAthlete {
            association: "association".into(),
        },
        SourceNamespace::TimerAthlete {
            provider: "timer".into(),
        },
        SourceNamespace::AthleticNet {
            kind: "athlete".into(),
        },
    ] {
        let mut fixture = Fixture::new()?;
        let mut claim = binding("a", "1")?;
        claim.source.namespace = namespace;
        let _ids = fixture.append(HistoryRequest::Establish {
            binding: claim,
            evidence: evidence("source", CandidateEvidenceKind::SourceBound, &["a"])?,
        })?;
        roundtrip(&fixture)?;
    }
    Ok(())
}

#[test]
fn dated_supersession_preserves_bytes_and_checked_date_ranges() -> TestResult {
    use crate::model::{ProfileDate, ProfileDateRangeV1};
    let mut fixture = Fixture::new()?;
    for (id, from, through, prior) in [
        ("a", "2020-01-01", "2020-12-31", None),
        ("b", "2026-01-01", "2026-12-31", Some("binding:a")),
    ] {
        let mut claim = binding(id, "1")?;
        claim.effective = EffectiveScope::Closed(ProfileDateRangeV1::new(
            ProfileDate::parse(from)?,
            ProfileDate::parse(through)?,
        )?);
        claim.supersedes = prior.map(str::to_owned);
        let _ids = fixture.append(HistoryRequest::Establish {
            binding: claim,
            evidence: evidence(id, CandidateEvidenceKind::SourceBound, &[id])?,
        })?;
    }
    roundtrip(&fixture)
}

#[test]
fn unknown_and_duplicate_fields_are_refused_at_every_object_boundary() -> TestResult {
    let fixture = complete()?;
    let original = serde_json::to_value(&fixture.records)?;
    for pointer in [
        "/0",
        "/0/previous",
        "/0/request/EstablishMatchedGroup",
        "/0/request/EstablishMatchedGroup/evidence",
        "/0/request/EstablishMatchedGroup/bindings/0",
        "/0/request/EstablishMatchedGroup/bindings/0/source",
        "/5/request/Revoke",
        "/6/request/Split",
        "/6/request/Split/partitions/0",
    ] {
        let mut value = original.clone();
        value
            .pointer_mut(pointer)
            .and_then(serde_json::Value::as_object_mut)
            .ok_or("missing object")?
            .insert("unexpected".into(), serde_json::json!(true));
        check!(
            error(&serde_json::to_vec(&value)?, &fixture, ReplayLimits::SMALL)?
                .contains("unknown field")
        );
    }
    let raw = serde_json::to_string(&fixture.records)?;
    for needle in [
        "\"version\":1",
        "\"sequence\":0",
        "\"identifier\":\"1\"",
        "\"application_id\":\"matched\"",
        "\"supersedes\":null",
    ] {
        let repeated = raw.replacen(needle, &format!("{needle},{needle}"), 1);
        check!(
            error(repeated.as_bytes(), &fixture, ReplayLimits::SMALL)?.contains("duplicate field")
        );
    }
    Ok(())
}

#[test]
fn missing_required_fields_do_not_default_history_meaning() -> TestResult {
    let fixture = Fixture::one()?;
    for field in [
        "version",
        "installation",
        "sequence",
        "previous",
        "request",
        "digest",
    ] {
        let mut value = serde_json::to_value(&fixture.records)?;
        value
            .pointer_mut("/0")
            .and_then(serde_json::Value::as_object_mut)
            .ok_or("missing record")?
            .remove(field)
            .ok_or("missing field")?;
        check!(
            error(&serde_json::to_vec(&value)?, &fixture, ReplayLimits::SMALL)?
                .contains("missing field")
        );
    }
    Ok(())
}

#[test]
fn unknown_version_is_refused_before_decoding_its_request() -> TestResult {
    let fixture = Fixture::one()?;
    let raw = br#"[{"version":2,"request":{"unrecognized":null}}]"#;
    check!(error(raw, &fixture, ReplayLimits::SMALL)?.contains("unsupported history version"));
    Ok(())
}

#[test]
fn invalid_store_public_ids_and_ranges_refuse_checked_decoding() -> TestResult {
    let fixture = complete()?;
    for (pointer, replacement) in [
        ("/0/installation", serde_json::json!("invalid")),
        (
            "/1/request/Bind/owner",
            serde_json::json!("ath_public_v1_BAD"),
        ),
        ("/0/previous/digest", serde_json::json!("A".repeat(64))),
        (
            "/0/request/EstablishMatchedGroup/bindings/0/effective",
            serde_json::json!({"Closed":{"from":"2026-02-30","through":"2026-03-01"}}),
        ),
        (
            "/0/request/EstablishMatchedGroup/bindings/0/effective",
            serde_json::json!({"Closed":{"from":"2026-03-02","through":"2026-03-01"}}),
        ),
    ] {
        let mut value = serde_json::to_value(&fixture.records)?;
        replace(&mut value, pointer, replacement)?;
        check!(!error(&serde_json::to_vec(&value)?, &fixture, ReplayLimits::SMALL)?.is_empty());
    }
    Ok(())
}
