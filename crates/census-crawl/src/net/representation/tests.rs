use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[test]
fn canonical_headers_sort_and_lowercase_the_delivered_representation() -> TestResult {
    let headers = RepresentationHeaders::canonical(&[
        ("Accept".to_string(), "application/json".to_string()),
        ("Anettokens".to_string(), "token-value".to_string()),
    ])?;
    check!(eq;
        headers.entries(),
        [
            ("accept".to_string(), "application/json".to_string()),
            ("anettokens".to_string(), "token-value".to_string()),
        ]
        .as_slice()
    );
    check!(eq; headers.identity(), "accept=application/json\u{1f}anettokens=token-value\u{1f}");
    Ok(())
}

#[test]
fn an_empty_representation_has_an_empty_identity() -> TestResult {
    let headers = RepresentationHeaders::canonical(&[])?;
    check!(headers.is_empty(), "empty source yields no entries");
    check!(eq; headers.identity(), "");
    Ok(())
}

#[test]
fn a_prohibited_header_is_refused_rather_than_dropped() -> TestResult {
    for name in ["Cookie", "Authorization", "cf-clearance"] {
        let refused =
            match RepresentationHeaders::canonical(&[(name.to_string(), "value".to_string())]) {
                Err(FetchError::Policy { detail }) => detail,
                other => {
                    return Err(format!("{name} must be refused by policy, got: {other:?}").into())
                }
            };
        check!(
            refused.contains(name),
            "the refusal names {name}: {refused}"
        );
    }
    Ok(())
}

#[test]
fn a_compact_user_agent_is_allowed() -> TestResult {
    let result =
        RepresentationHeaders::canonical(&[("user-agent".to_string(), "Mozilla/5.0".to_string())]);
    check!(
        result.is_ok(),
        "a compact User-Agent must be allowed: {result:?}"
    );
    Ok(())
}

#[test]
fn a_compact_user_agent_is_part_of_the_declared_identity() -> TestResult {
    let headers = RepresentationHeaders::canonical(&[(
        "User-Agent".to_string(),
        "Mozilla/5.0 Chrome/126.0 census-service/0.1".to_string(),
    )])?;
    check!(eq;
        headers.entries(),
        [(
            "user-agent".to_string(),
            "Mozilla/5.0 Chrome/126.0 census-service/0.1".to_string()
        )]
        .as_slice()
    );
    check!(eq;
        headers.identity(),
        "user-agent=Mozilla/5.0 Chrome/126.0 census-service/0.1\u{1f}"
    );
    Ok(())
}

#[test]
fn a_comment_form_user_agent_is_refused_and_the_refusal_names_the_value() -> TestResult {
    for value in [
        "census-service/0.1 (independent HS track & field research collector; polite; contact: repo owner)",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36 census-service/0.1",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/126.0 Safari/537.36",
    ] {
        let refused = match RepresentationHeaders::canonical(&[(
            "user-agent".to_string(),
            value.to_string(),
        )]) {
            Err(FetchError::Policy { detail }) => detail,
            other => {
                return Err(format!("{value} must be refused by policy, got: {other:?}").into())
            }
        };
        check!(
            refused.contains("parenthesized comment form"),
            "the refusal states the compact agent rule: {refused}"
        );
        check!(
            refused.contains(value),
            "the refusal names the offending value {value}: {refused}"
        );
    }
    Ok(())
}

#[test]
fn an_empty_oversized_multiline_padded_or_non_printable_user_agent_is_refused() -> TestResult {
    let over = "v".repeat(MAX_USER_AGENT_BYTES + 1);
    for value in [
        "".to_string(),
        " ".to_string(),
        " Mozilla/5.0".to_string(),
        "Mozilla/5.0 ".to_string(),
        "Mozilla/5.0\r\nAccept: text/html".to_string(),
        "M\u{f6}bius/5.0".to_string(),
        "Mozilla\u{7f}/5.0".to_string(),
        over,
    ] {
        let refused =
            match RepresentationHeaders::canonical(&[("user-agent".to_string(), value.clone())]) {
                Err(FetchError::Policy { detail }) => detail,
                other => {
                    return Err(
                        format!("{value:?} must be refused by policy, got: {other:?}").into(),
                    )
                }
            };
        check!(
            refused.contains(&format!("{value:?}")),
            "the refusal names the offending value {value:?}: {refused}"
        );
    }
    Ok(())
}

#[test]
fn a_user_agent_at_the_compact_ceiling_is_allowed() -> TestResult {
    let at_ceiling = "v".repeat(MAX_USER_AGENT_BYTES);
    let headers =
        RepresentationHeaders::canonical(&[("user-agent".to_string(), at_ceiling.clone())])?;
    check!(eq; headers.entries(), [("user-agent".to_string(), at_ceiling)].as_slice());
    Ok(())
}

#[test]
fn a_repeated_header_name_is_refused() -> TestResult {
    let refused = match RepresentationHeaders::canonical(&[
        ("accept".to_string(), "application/json".to_string()),
        ("Accept".to_string(), "text/html".to_string()),
    ]) {
        Err(FetchError::Policy { detail }) => detail,
        other => return Err(format!("a repeated name must be refused, got: {other:?}").into()),
    };
    check!(
        refused.contains("accept"),
        "the refusal names the header: {refused}"
    );
    Ok(())
}

#[test]
fn the_header_count_and_sizes_are_bounded() -> TestResult {
    let many: Vec<(String, String)> = (0..=MAX_REPRESENTATION_HEADERS)
        .map(|index| (format!("accept-{index}"), "value".to_string()))
        .collect();
    check!(
        matches!(
            RepresentationHeaders::canonical(&many),
            Err(FetchError::Policy { .. })
        ),
        "a representation above the header ceiling is refused"
    );
    check!(
        matches!(
            RepresentationHeaders::canonical(&[(
                "accept".to_string(),
                "v".repeat(MAX_REPRESENTATION_VALUE_BYTES + 1)
            )]),
            Err(FetchError::Policy { .. })
        ),
        "a value above the byte ceiling is refused"
    );
    Ok(())
}
