use super::*;
use fixture::*;

#[test]
fn shell_checks_encoded_bytes_before_constructing_the_deserializer() -> TestResult {
    let fixture = Fixture::one()?;
    let raw = serde_json::to_vec(&fixture.records)?;
    let limit = raw.len().checked_sub(1).ok_or("empty encoded history")?;
    match decode(&raw, &fixture, ReplayLimits::SMALL, limit) {
        Err(error) => check!(eq; error.to_string(), "shell encoded history byte limit"),
        Ok(_) => return Err("shell admitted oversized bytes".into()),
    }
    check!(decode(&raw, &fixture, ReplayLimits::SMALL, raw.len()).is_ok());
    Ok(())
}

#[test]
fn shell_end_rejects_a_second_document_and_trailing_garbage() -> TestResult {
    let fixture = Fixture::one()?;
    let original = serde_json::to_vec(&fixture.records)?;
    for suffix in [b"[]".as_slice(), b" trailing".as_slice()] {
        let mut raw = original.clone();
        raw.extend_from_slice(suffix);
        check!(error(&raw, &fixture, ReplayLimits::SMALL)?.contains("trailing characters"));
    }
    Ok(())
}

#[test]
fn record_limit_refuses_before_decoding_an_extra_record() -> TestResult {
    let fixture = Fixture::one()?;
    let original = serde_json::to_string(&fixture.records)?;
    let prefix = original.strip_suffix(']').ok_or("missing closing array")?;
    let raw = format!("{prefix},{{\"intentionally incomplete\"");
    let limits = ReplayLimits {
        records: 1,
        ..ReplayLimits::SMALL
    };
    check!(error(raw.as_bytes(), &fixture, limits)?.contains("history collection limit"));
    let empty = Fixture::new()?;
    check!(decode(
        b"[]",
        &empty,
        ReplayLimits {
            records: 0,
            ..ReplayLimits::SMALL
        },
        2
    )
    .is_ok());
    Ok(())
}

#[test]
fn individual_collections_refuse_before_retaining_extra_elements() -> TestResult {
    use super::super::atoms::{CandidateNode, NumberNode, TextNode};
    use super::super::claims::BindingNode;
    use super::super::containers::{Affected, Bindings, ListNode, Members};
    use super::super::corrections::PartitionNode;
    let limits = ReplayLimits {
        members_per_operation: 0,
        affected_record_keys: 0,
        ..ReplayLimits::SMALL
    };
    refused_list::<ListNode<BindingNode, Bindings>>(limits)?;
    refused_list::<ListNode<CandidateNode, Members>>(limits)?;
    refused_list::<ListNode<TextNode, Affected>>(limits)?;
    refused_list::<ListNode<TextNode, Members>>(limits)?;
    refused_list::<ListNode<NumberNode, Members>>(limits)?;
    refused_list::<ListNode<PartitionNode, Members>>(limits)
}

fn refused_list<N: super::super::containers::Node>(limits: ReplayLimits) -> TestResult {
    let mut meter = super::super::containers::Meter::new(limits);
    let mut deserializer = serde_json::Deserializer::from_str("[{\"intentionally incomplete\"");
    let result =
        super::super::containers::NodeSeed::<N>::new(&mut meter).deserialize(&mut deserializer);
    match result {
        Err(error) => check!(error.to_string().contains("history collection limit")),
        Ok(_) => return Err("collection retained an excess element".into()),
    }
    Ok(())
}

#[test]
fn exact_cumulative_request_text_admission_matches_existing_history_budget() -> TestResult {
    let fixture = complete()?;
    let text = fixture.records.iter().try_fold(0_usize, |total, record| {
        let bytes =
            crate::public_identity_history::budget::request(&record.request, ReplayLimits::SMALL)?;
        total
            .checked_add(bytes)
            .ok_or(HistoryError::CounterOverflow)
    })?;
    let raw = serde_json::to_vec(&fixture.records)?;
    let exact = ReplayLimits {
        total_text_bytes: text,
        ..ReplayLimits::SMALL
    };
    check!(decode(&raw, &fixture, exact, raw.len()).is_ok());
    let less = ReplayLimits {
        total_text_bytes: text.checked_sub(1).ok_or("no text")?,
        ..exact
    };
    check!(error(&raw, &fixture, less)?.contains("history total text byte limit"));
    Ok(())
}

#[test]
fn decoded_text_limits_include_escaped_strings_and_preserve_non_ascii() -> TestResult {
    let mut fixture = Fixture::new()?;
    let _ids = fixture.append(establish("opaque λ candidate", "17")?)?;
    let raw = serde_json::to_string(&fixture.records)?;
    let escaped = raw.replace('λ', "\\u03bb");
    let decoded = decode(
        escaped.as_bytes(),
        &fixture,
        ReplayLimits::SMALL,
        escaped.len(),
    )?;
    check!(eq; serde_json::to_vec(decoded.records())?, serde_json::to_vec(&fixture.records)?);
    let mut value = serde_json::to_value(&fixture.records)?;
    replace(
        &mut value,
        "/0/request/Establish/binding/candidate",
        serde_json::json!("x".repeat(513)),
    )?;
    check!(
        error(&serde_json::to_vec(&value)?, &fixture, ReplayLimits::SMALL)?
            .contains("history text byte limit")
    );
    Ok(())
}

#[test]
fn empty_and_control_containing_opaque_candidate_strings_are_refused() -> TestResult {
    let fixture = Fixture::one()?;
    for candidate in ["", "candidate\n", "candidate\0", "candidate\u{7f}"] {
        let mut value = serde_json::to_value(&fixture.records)?;
        replace(
            &mut value,
            "/0/request/Establish/binding/candidate",
            serde_json::json!(candidate),
        )?;
        check!(
            error(&serde_json::to_vec(&value)?, &fixture, ReplayLimits::SMALL)?
                .contains("invalid history text")
        );
    }
    Ok(())
}

#[test]
fn source_namespace_payloads_refuse_unknown_and_repeated_fields() -> TestResult {
    let fixture = Fixture::one()?;
    for namespace in [
        r#"{"association_athlete":{"association":"x","extra":1}}"#,
        r#"{"association_athlete":{"association":"x","association":"x"}}"#,
        r#"{"timer_athlete":{"provider":"x","kind":"athlete"}}"#,
        r#"{"athletic_net":{"kind":"team"}}"#,
    ] {
        let raw =
            serde_json::to_string(&fixture.records)?.replacen("\"tfrrs_athlete\"", namespace, 1);
        check!(!error(raw.as_bytes(), &fixture, ReplayLimits::SMALL)?.is_empty());
    }
    Ok(())
}
