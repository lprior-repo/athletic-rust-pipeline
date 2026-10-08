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
    for name in ["Cookie", "Authorization", "cf-clearance", "User-Agent"] {
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
