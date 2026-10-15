use super::*;
use fixture::*;

#[test]
fn decoded_payload_hashes_are_revalidated_before_projection_exposure() -> TestResult {
    let fixture = Fixture::one()?;
    let mut value = serde_json::to_value(&fixture.records)?;
    replace(
        &mut value,
        "/0/request/Establish/binding/source/identifier",
        serde_json::json!("different"),
    )?;
    check!(
        error(&serde_json::to_vec(&value)?, &fixture, ReplayLimits::SMALL)?
            .contains("payload digest differs")
    );
    Ok(())
}

#[test]
fn installation_sequence_previous_and_final_heads_are_independent_replay_fences() -> TestResult {
    let fixture = Fixture::one()?;
    for (pointer, replacement, expected) in [
        (
            "/0/installation",
            serde_json::json!("2".repeat(64)),
            "another installation",
        ),
        (
            "/0/sequence",
            serde_json::json!(2),
            "next committed position",
        ),
        (
            "/0/previous/digest",
            serde_json::json!("a".repeat(64)),
            "previous or final head differs",
        ),
    ] {
        let mut value = serde_json::to_value(&fixture.records)?;
        replace(&mut value, pointer, replacement)?;
        check!(
            error(&serde_json::to_vec(&value)?, &fixture, ReplayLimits::SMALL)?.contains(expected)
        );
    }
    let mut wrong = Fixture::one()?;
    wrong.head.digest = "b".repeat(64);
    check!(error(
        &serde_json::to_vec(&fixture.records)?,
        &wrong,
        ReplayLimits::SMALL
    )?
    .contains("previous or final head differs"));
    Ok(())
}

#[test]
fn validly_encoded_but_unsupported_graph_transitions_are_not_exposed() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _ids = fixture.append(establish("a", "same-source")?)?;
    let _ids = fixture.append(establish("b", "same-source")?)?;
    let raw = serde_json::to_vec(&fixture.records)?;
    check!(error(&raw, &fixture, ReplayLimits::SMALL)?.contains("source identifier reuse"));
    Ok(())
}

#[test]
fn replay_enforces_graph_identity_and_traversal_limits_after_decode_admission() -> TestResult {
    let fixture = complete()?;
    let raw = serde_json::to_vec(&fixture.records)?;
    for (limits, expected) in [
        (
            ReplayLimits {
                identities: 0,
                ..ReplayLimits::SMALL
            },
            "identities",
        ),
        (
            ReplayLimits {
                traversal_steps: 0,
                ..ReplayLimits::SMALL
            },
            "traversal steps",
        ),
        (
            ReplayLimits {
                graph_steps: 1,
                ..ReplayLimits::SMALL
            },
            "graph steps",
        ),
    ] {
        check!(error(&raw, &fixture, limits)?.contains(expected));
    }
    Ok(())
}

#[test]
fn unknown_request_tags_and_noncanonical_member_order_are_refused() -> TestResult {
    let fixture = complete()?;
    let raw = serde_json::to_string(&fixture.records)?;
    check!(error(
        raw.replacen("\"EstablishMatchedGroup\"", "\"FutureOperation\"", 1)
            .as_bytes(),
        &fixture,
        ReplayLimits::SMALL
    )?
    .contains("unknown variant"));
    let mut value = serde_json::to_value(&fixture.records)?;
    value
        .pointer_mut("/0/request/EstablishMatchedGroup/evidence/members")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("missing members")?
        .reverse();
    check!(
        error(&serde_json::to_vec(&value)?, &fixture, ReplayLimits::SMALL)?
            .contains("noncanonical request order")
    );
    Ok(())
}
